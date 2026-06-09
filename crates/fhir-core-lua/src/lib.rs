use fhir_core::ping;
use mlua::{Lua, Result, Table};

#[mlua::lua_module]
fn fhir_core(lua: &Lua) -> Result<Table> {
    let exports = lua.create_table()?;
    exports.set("ping", lua.create_function(|_, ()| Ok(ping()))?)?;
    Ok(exports)
}
