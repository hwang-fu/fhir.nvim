use fhir_core::{Engine, Resolve};
use mlua::{Function, Lua, Result, Table};

// a Lua callback wearing the engine's resolver trait; a throwing callback or
// invalid json means "unresolved", never an aborted evaluation
struct LuaResolver(Function);

impl Resolve for LuaResolver {
    fn resolve(&self, reference: &str) -> Option<serde_json::Value> {
        let text: Option<String> = self.0.call(reference.to_string()).ok().flatten();
        text.and_then(|s| serde_json::from_str(&s).ok())
    }
}

#[mlua::lua_module]
fn fhir_core(lua: &Lua) -> Result<Table> {
    let exports = lua.create_table()?;
    exports.set(
        "eval",
        lua.create_function(
            |_, (json, expr, resolver): (String, String, Option<Function>)| {
                let engine = match resolver {
                    Some(f) => Engine::new().with_resolver(Box::new(LuaResolver(f))),
                    None => Engine::new(),
                };
                // Lua convention: result on success, nil + message on failure
                Ok(match engine.evaluate_json(&json, &expr) {
                    Ok(result) => (Some(result), None),
                    Err(e) => (None, Some(e.to_string())),
                })
            },
        )?,
    )?;
    Ok(exports)
}
