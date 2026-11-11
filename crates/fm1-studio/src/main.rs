//! FM-1 Studio: native firmware workstation, using the upstream emulator.
mod audio;
mod catalog;
mod desktop;
mod midi;
mod persistence;
mod runtime;
mod theme;
mod ui;
fn main() -> eframe::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|a| a == "--bench") {
        let result = args
            .get(2)
            .ok_or("usage: fm1-studio --bench PACKAGE.fwsc [milliseconds]".into())
            .and_then(|p| {
                runtime::benchmark(
                    p.into(),
                    args.get(3).and_then(|s| s.parse().ok()).unwrap_or(3000),
                )
            });
        if let Err(e) = result {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if args.get(1).is_some_and(|a| a == "--version") {
        println!("FM-1 Studio {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.get(1).is_some_and(|a| a == "--devices") {
        println!(
            "Audio outputs: {:?}\nMIDI inputs: {:?}",
            audio::devices(),
            midi::port_names()
        );
        return Ok(());
    }
    if args.get(1).is_some_and(|a| a == "--runtime-smoke") {
        let result = if args.len() == 5 {
            runtime::smoke(
                args[2].clone().into(),
                args[3].clone().into(),
                args[4].clone().into(),
            )
        } else {
            Err("usage: --runtime-smoke FIRST.fwsc SECOND.fwsc ISOLATED_DATA_DIR".into())
        };
        if let Err(e) = result {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if args
        .get(1)
        .is_some_and(|a| a == "--check-catalog" || a == "--fetch")
    {
        let result = (|| {
            let path = args.get(2).ok_or("catalog path required")?;
            let c = catalog::parse(&persistence::read_bounded(
                std::path::Path::new(path),
                4 * 1024 * 1024,
            )?)?;
            if args[1] == "--check-catalog" {
                println!("{} catalog entries validated", c.firmwares.len());
                return Ok::<(), String>(());
            }
            let id = args.get(3).ok_or("firmware ID required")?;
            let root = args.get(4).ok_or("cache directory required")?;
            let e = c
                .firmwares
                .into_iter()
                .find(|e| &e.id == id)
                .ok_or("unknown firmware ID")?;
            let job = catalog::download(root.into(), e);
            while let Ok(update) = job.rx.recv() {
                match update {
                    catalog::Update::Downloaded(p) => {
                        println!("Verified: {}", p.display());
                        return Ok(());
                    }
                    catalog::Update::Error(e) => return Err(e),
                    catalog::Update::Progress(s) => eprintln!("{s}"),
                    _ => {}
                }
            }
            Err("download worker stopped".into())
        })();
        if let Err(e) = result {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if args.get(1).is_some_and(|a| a == "--install-desktop") {
        match desktop::install() {
            Ok(s) => println!("{s}"),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    let initial = args.get(1).map(std::path::PathBuf::from);
    eframe::run_native(
        "FM-1 Studio",
        eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_inner_size([1460.0, 960.0])
                .with_min_inner_size([1040.0, 720.0])
                .with_app_id("com.fm1studio.Studio")
                .with_icon(
                    eframe::icon_data::from_png_bytes(include_bytes!(
                        "../../../packaging/studio/fm1-studio.png"
                    ))
                    .unwrap_or_default(),
                ),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| Ok(Box::new(ui::Studio::new(cc, initial)))),
    )
}
