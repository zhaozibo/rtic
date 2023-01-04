use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::analyze::Analysis;
use crate::syntax::ast::App;

mod assertions;
mod async_dispatchers;
// mod dispatchers;
mod hardware_tasks;
mod idle;
mod init;
mod local_resources;
mod local_resources_struct;
mod module;
// mod monotonic;
mod post_init;
mod pre_init;
mod shared_resources;
mod shared_resources_struct;
mod software_tasks;
// mod timer_queue;
mod util;

#[allow(clippy::too_many_lines)]
pub fn app(app: &App, analysis: &Analysis) -> TokenStream2 {
    let mut mod_app = vec![];
    let mut mains = vec![];
    let mut root = vec![];
    let mut user = vec![];

    // Generate the `main` function
    let assertion_stmts = assertions::codegen(app, analysis);

    let pre_init_stmts = pre_init::codegen(app, analysis);

    let (mod_app_init, root_init, user_init, call_init) = init::codegen(app, analysis);

    let post_init_stmts = post_init::codegen(app, analysis);

    let (mod_app_idle, root_idle, user_idle, call_idle) = idle::codegen(app, analysis);

    user.push(quote!(
        #user_init

        #user_idle
    ));

    root.push(quote!(
        #(#root_init)*

        #(#root_idle)*
    ));

    mod_app.push(quote!(
        #mod_app_init

        #(#mod_app_idle)*
    ));

    let main = util::suffixed("main");
    mains.push(quote!(
        #[doc(hidden)]
        mod rtic_ext {
            use super::*;
            #[no_mangle]
            unsafe extern "C" fn #main() -> ! {
                #(#assertion_stmts)*

                #(#pre_init_stmts)*

                #[inline(never)]
                fn __rtic_init_resources<F>(f: F) where F: FnOnce() {
                    f();
                }

                // Wrap late_init_stmts in a function to ensure that stack space is reclaimed.
                __rtic_init_resources(||{
                    #call_init

                    #(#post_init_stmts)*
                });

                #call_idle
            }
        }
    ));

    let (mod_app_shared_resources, mod_shared_resources) = shared_resources::codegen(app, analysis);
    let (mod_app_local_resources, mod_local_resources) = local_resources::codegen(app, analysis);

    let (mod_app_hardware_tasks, root_hardware_tasks, user_hardware_tasks) =
        hardware_tasks::codegen(app, analysis);

    let (mod_app_software_tasks, root_software_tasks, user_software_tasks) =
        software_tasks::codegen(app, analysis);

    let mod_app_async_dispatchers = async_dispatchers::codegen(app, analysis);
    let user_imports = &app.user_imports;
    let user_code = &app.user_code;
    let name = &app.name;
    let device = &app.args.device;

    let rt_err = util::rt_err_ident();

    quote!(
        /// The RTIC application module
        pub mod #name {
            /// Always include the device crate which contains the vector table
            use #device as #rt_err;

            #(#user_imports)*

            /// User code from within the module
            #(#user_code)*
            /// User code end

            #(#user)*

            #(#user_hardware_tasks)*

            #(#user_software_tasks)*

            #(#root)*

            #mod_shared_resources

            #mod_local_resources

            #(#root_hardware_tasks)*

            #(#root_software_tasks)*

            /// app module
            #(#mod_app)*

            #(#mod_app_shared_resources)*

            #(#mod_app_local_resources)*

            #(#mod_app_hardware_tasks)*

            #(#mod_app_software_tasks)*

            #(#mod_app_async_dispatchers)*

            #(#mains)*
        }
    )
}
