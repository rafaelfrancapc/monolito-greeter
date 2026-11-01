mod auth;
mod background;
mod config;
mod policy;
mod ui;

use gtk::prelude::*;
use std::{cell::Cell, rc::Rc};

fn main() -> glib::ExitCode {
    let args: Vec<_> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Monolito Greeter\n  --demo         Janela isolada; nunca autentica no sistema\n  --smoke-test   Validação GTK automatizada; requer --demo\n  --help         Mostra esta ajuda\n\nSem --demo, a conexão com o LightDM é obrigatória.");
        return glib::ExitCode::SUCCESS;
    }
    let demo = args.iter().any(|a| a == "--demo");
    let smoke = args.iter().any(|a| a == "--smoke-test");
    if (smoke && !demo)
        || args
            .iter()
            .skip(1)
            .any(|a| !matches!(a.as_str(), "--demo" | "--smoke-test"))
    {
        eprintln!("Opções inválidas. Use --help. --smoke-test requer --demo.");
        return glib::ExitCode::FAILURE;
    }
    let config = match config::Config::load() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            return glib::ExitCode::FAILURE;
        }
    };
    let app = gtk::Application::builder()
        .application_id("org.monolito.Greeter")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    let config = Rc::new(config);
    let failed = Rc::new(Cell::new(false));
    let failure = failed.clone();
    app.connect_activate(move |app| {
        if let Some(window) = app.active_window() {
            window.present();
            return;
        }
        let backend = match auth::Backend::new(demo) {
            Ok(backend) => backend,
            Err(error) => {
                eprintln!("{error}");
                failure.set(true);
                app.quit();
                return;
            }
        };
        ui::install_css();
        let ui = ui::Greeter::build(app, backend, &config);
        let keep_alive = ui.clone();
        app.connect_shutdown(move |_| {
            keep_alive.clear_secrets();
        });
        ui.connect();
        ui.window.present();
        if !demo {
            ui.window.fullscreen();
        }
        if smoke {
            ui::smoke_test(ui, failure.clone());
        }
    });
    let result = app.run_with_args::<&str>(&[]);
    if failed.get() {
        glib::ExitCode::FAILURE
    } else {
        result
    }
}
