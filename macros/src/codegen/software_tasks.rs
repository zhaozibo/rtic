use crate::syntax::{ast::App, Context};
use crate::{
    analyze::Analysis,
    codegen::{local_resources_struct, module, shared_resources_struct, util},
};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub fn codegen(
    app: &App,
    analysis: &Analysis,
) -> (
    // mod_app_software_tasks -- free queues, buffers and `${task}Resources` constructors
    Vec<TokenStream2>,
    // root_software_tasks -- items that must be placed in the root of the crate:
    // - `${task}Locals` structs
    // - `${task}Resources` structs
    // - `${task}` modules
    Vec<TokenStream2>,
    // user_software_tasks -- the `#[task]` functions written by the user
    Vec<TokenStream2>,
) {
    let mut mod_app = vec![];
    let mut root = vec![];
    let mut user_tasks = vec![];

    // Any task
    for (name, task) in app.software_tasks.iter() {
        let executor_ident = util::executor_run_ident(name);
        mod_app.push(quote!(
            #[allow(non_camel_case_types)]
            #[allow(non_upper_case_globals)]
            #[doc(hidden)]
            static #executor_ident: core::sync::atomic::AtomicBool =
                core::sync::atomic::AtomicBool::new(false);
        ));

        // `${task}Resources`
        let mut shared_needs_lt = false;
        let mut local_needs_lt = false;

        // `${task}Locals`
        if !task.args.local_resources.is_empty() {
            let (item, constructor) = local_resources_struct::codegen(
                Context::SoftwareTask(name),
                &mut local_needs_lt,
                app,
            );

            root.push(item);

            mod_app.push(constructor);
        }

        if !task.args.shared_resources.is_empty() {
            let (item, constructor) = shared_resources_struct::codegen(
                Context::SoftwareTask(name),
                &mut shared_needs_lt,
                app,
            );

            root.push(item);

            mod_app.push(constructor);
        }

        if !&task.is_extern {
            let context = &task.context;
            let attrs = &task.attrs;
            let cfgs = &task.cfgs;
            let stmts = &task.stmts;
            let context_lifetime = if shared_needs_lt || local_needs_lt {
                quote!(<'static>)
            } else {
                quote!()
            };

            user_tasks.push(quote!(
                #(#attrs)*
                #(#cfgs)*
                #[allow(non_snake_case)]
                async fn #name(#context: #name::Context #context_lifetime) {
                    use rtic::Mutex as _;
                    use rtic::mutex::prelude::*;

                    #(#stmts)*
                }
            ));
        }

        root.push(module::codegen(
            Context::SoftwareTask(name),
            shared_needs_lt,
            local_needs_lt,
            app,
            analysis,
        ));
    }

    (mod_app, root, user_tasks)
}
