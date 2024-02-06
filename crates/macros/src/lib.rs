use proc_macro::TokenStream;

/// Register a function as an entrypoint of a Lua module and generate a test that
/// calls that function from Lua.
///
/// If the `test` feature flag is disabled, this macro does not do anything.
///
/// ```no_run
/// #[cfg(test)]
/// fn lua_eval(lua_code: &str) -> std::process::Output {
///     std::process::Command::new("lua")
///         .args(["-e", lua_code])
///         .output()
///         .expect("failed to execute process")
/// }
/// 
/// #[lua_module_test(lua_eval)]
/// fn test(lua: &mlua::Lua) -> mlua::Result<()> {
///     lua.globals()
///         .get::<_, LuaFunction>("print")?
///         .call("hello")?;
///     Ok(())
/// }
/// ```
#[cfg(feature = "test")]
#[proc_macro_attribute]
pub fn lua_module_test(attr: TokenStream, item: TokenStream) -> TokenStream {
    use proc_macro2::{Ident, Span};
    use quote::quote;
    use syn::parse_macro_input;

    let spawner = parse_macro_input!(attr as syn::Expr);
    let item = parse_macro_input!(item as syn::ItemFn);

    let body = item.block;
    let sig = item.sig;
    let output = sig.output.clone();
    let ident = sig.ident.clone();
    let generics = sig.generics.clone();
    let inputs = sig.inputs.clone();

    let hash = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        body.hash(&mut hasher);
        hasher.finish()
    };
    let lua_ident = Ident::new(&format!("{ident}{hash}"), Span::call_site());

    let lua_code = {
        let dll = std::env::current_dir()
            .unwrap()
            .join("lua")
            .join("lunest")
            .join("lunest.dll");
        let dll = dll.to_string_lossy().to_string().replace('\\', "/");
        format!("assert(package.loadlib('{dll}', 'luaopen_{lua_ident}'))()")
    };

    let generated = quote! {
        #[::mlua::lua_module]
        fn #lua_ident #generics (#inputs) -> ::mlua::Result<::mlua::Value> {
            use ::std::process::exit;
            let body = || #output #body;
            ::scopeguard::defer_on_unwind! {
                exit(1);
            }
            body()?;
            exit(0);
        }

        #[test]
        fn #ident() {
            use ::std::string::String;

            let out = #spawner(#lua_code);

            if !out.stdout.is_empty() {
                println!("```stdout\n{}\n```", String::from_utf8_lossy(&out.stdout));
            }
            if !out.stderr.is_empty() {
                println!("```stderr\n{}\n```", String::from_utf8_lossy(&out.stderr));
            }
            match out.status.code() {
                Some(0) => (),
                Some(n) => panic!("exit with status code {n}"),
                None => panic!("exit without status code")
            }
        }
    };

    TokenStream::from(generated)
}

#[cfg(not(feature = "test"))]
#[proc_macro_attribute]
pub fn lua_module_test(_attr: TokenStream, _item: TokenStream) -> TokenStream {
    TokenStream::new()
}
