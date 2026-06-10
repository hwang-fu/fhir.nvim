use fhir_core::evaluate_json;
use mlua::{Lua, Result, Table};

#[mlua::lua_module]
fn fhir_core(lua: &Lua) -> Result<Table> {
    let exports = lua.create_table()?;
    exports.set(
        "eval",
        lua.create_function(|_, (json, expr): (String, String)| {
            // Lua convention: result on success, nil + message on failure
            Ok(match evaluate_json(&json, &expr) {
                Ok(result) => (Some(result), None),
                Err(e) => (None, Some(e.to_string())),
            })
        })?,
    )?;
    Ok(exports)
}
