use anyhow::Result;
use clap::Parser;
use mythos::faction::Faction;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "vaultforge", about = "VaultForge — myth-os Rust engine")]
struct Cli {
    /// Path to the vault directory
    #[arg(short, long, default_value = ".")]
    vault: PathBuf,

    /// Faction theme (luminarite|venturan|sylvanid|hydralis|syntaran)
    #[arg(short, long)]
    theme: Option<String>,

    /// Path to a PNG skin pack file (overrides --theme)
    #[arg(long)]
    skin: Option<PathBuf>,

    /// Run without UI
    #[arg(long)]
    headless: bool,

    /// Start the storefront REST API on this port (implies headless).
    /// Bare `--serve` defaults to 7878 to avoid clashing with common
    /// dev services (e.g. Ollama on 8080/11434).
    #[arg(long, num_args = 0..=1, default_missing_value = "7878")]
    serve: Option<u16>,

    /// Ask Quantum Quill, the vault's agent assistant, a single question
    /// and print the reply (implies headless). Provider/model are resolved
    /// from the environment — see the `quill` crate docs.
    #[arg(long, value_name = "PROMPT")]
    ask: Option<String>,

    /// Save a .qgenesis manifest to the vault directory after boot
    #[arg(long)]
    save_manifest: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    println!("╔══════════════════════════════════════╗");
    println!("║     VaultForge  ·  myth-os  v0.1     ║");
    println!("╚══════════════════════════════════════╝\n");

    // ── Quantum Quill (agent assistant) ───────────────────────────────────────
    // A quick `--ask` shouldn't need to boot the whole vault.
    if let Some(prompt) = &cli.ask {
        use std::io::Write;
        let router = quill::QuillRouter::resolve(&cli.vault)?;
        println!("Quantum Quill · via {}\n", router.provider().label());
        let result = router.ask_stream(prompt, |chunk| {
            print!("{chunk}");
            let _ = std::io::stdout().flush();
        });
        println!();
        if let Err(e) = result {
            eprintln!("Quill error: {e}");
            std::process::exit(1);
        }
        return Ok(());
    }

    // ── Vaultmap ──────────────────────────────────────────────────────────────
    match qgcp::VaultMap::load(&cli.vault) {
        Ok(vm) => {
            let i = vm.inner();
            println!("Vault:    {} — {}", i.repo, i.description);
            println!("Tiers:    {}", i.sovereign_tiers.join(" · "));
            println!("Branches: {}", i.branches.iter().map(|b| b.name.as_str()).collect::<Vec<_>>().join(", "));
        }
        Err(e) => println!("(vaultmap: {})", e),
    }
    println!();

    // ── Genesis ───────────────────────────────────────────────────────────────
    let genesis = mythos::genesis::GenesisContainer::new(
        myth_wire::new_id(),
        "Quantum Vault",
        "biospark-studios",
    );
    println!("Genesis:   {} ({})", genesis.name, &genesis.id[..8]);
    println!("BDna:      {}…", &genesis.bdna_signature[..16]);
    println!("Resonance: {:.1} Hz\n", genesis.resonance_hz);

    // ── Scrolls ───────────────────────────────────────────────────────────────
    let scroll_reg = vault_scrolls::ScrollRegistry::load(&cli.vault)?;
    println!("Scrolls: {}", scroll_reg.scrolls().len());
    for s in scroll_reg.scrolls() {
        let status = if s.is_expired() { "EXPIRED" } else { "valid  " };
        println!("  [{status}] {} — {:?} · remix:{} · {}", s.id, s.tier, s.remixable, s.bound_persona);
    }
    println!();

    // ── Capsules ──────────────────────────────────────────────────────────────
    let capsules = qgcp::load_capsules(&cli.vault)?;
    println!("Capsules: {}", capsules.len());
    for c in &capsules {
        println!("  {} — {:?} · remix:{} · bdna:{}", c.id, c.tier, c.remixable, c.bdna.weight());
    }
    println!();

    // ── Glyphs + Personas ─────────────────────────────────────────────────────
    let glyph_reg = vault_glyphs::GlyphRegistry::load(&cli.vault)?;
    println!("Glyphs: {}  |  Personas: {}\n", glyph_reg.glyphs().len(), personas::canonical_personas().len());

    // ── Plugin Slots ──────────────────────────────────────────────────────────
    let mut core = vault_core::VaultCore::new(&genesis.id);
    for slot in vault_core::default_plugin_slots() {
        core.register_plugin(slot);
    }

    // ── Theme ─────────────────────────────────────────────────────────────────
    let theme = if let Some(ref skin_path) = cli.skin {
        println!("Loading skin: {}", skin_path.display());
        theater::load_skin(skin_path)
            .unwrap_or_else(|e| { eprintln!("  skin error: {e}"); theater::VaultTheme::default_sylvanid() })
    } else {
        let faction = cli.theme.as_deref().and_then(Faction::from_str).unwrap_or(Faction::Sylvanid);
        theater::VaultTheme::from_faction(faction)
    };
    println!("Theme: {:?} · primary:{} bg:{}\n", theme.faction, theme.palette.primary, theme.palette.bg);

    // ── Remix Engine ──────────────────────────────────────────────────────────
    let mut remix_lineage = remix_engine::RemixLineage::new();
    let remixable: Vec<_> = capsules.iter().filter(|c| c.remixable).cloned().collect();
    if remixable.len() >= 2 {
        let source = remixable[0].clone();
        let mut derived = remixable[1].clone();
        derived.id = format!("{}-remix", derived.id);
        if let Ok(_pkt) = remix_lineage.register(&source, &mut derived, "vaultforge-boot") {
            println!("Remix demo: {} → {}", source.id, derived.id);
        }
    }

    // ── LoomEngine ────────────────────────────────────────────────────────────
    use loom_engine::{LoomEngine, warp::WarpThread};
    use myth_wire::WireType;

    let loom = LoomEngine::new();
    let mut threads = Vec::new();

    // Feed scroll titles as NAR threads
    for scroll in scroll_reg.scrolls() {
        if !scroll.is_expired() {
            threads.push(WarpThread::new(
                WireType::NAR,
                &scroll.id,
                serde_json::Value::String(scroll.title.clone()),
            ));
        }
    }
    // Feed a license gate result
    threads.push(WarpThread::new(WireType::LGC, "license-core", serde_json::json!(true)));
    // Feed DJ deck AUD at 432 Hz
    threads.push(
        WarpThread::new(WireType::AUD, "dj-deck", serde_json::json!(0.5))
            .with_resonance(432.0),
    );

    let result = loom.weave(threads);
    println!(
        "LoomEngine: {} threads → {} ops ({} dropped)",
        result.threads_in, result.ops.len(), result.threads_dropped
    );
    for op in &result.ops {
        println!("  {:?}", op);
    }
    println!();

    // ── .qgenesis persist ─────────────────────────────────────────────────────
    if cli.save_manifest {
        let mut manifest = qgcp::VaultManifest::new(&genesis.id, "Quantum Vault");
        manifest.persona_traits = personas::canonical_personas()
            .iter()
            .flat_map(|p| p.traits.clone())
            .collect();
        manifest.genesis = Some(genesis.clone());

        match qgcp::save_manifest(&manifest, &cli.vault) {
            Ok(path) => println!("Manifest saved: {}", path.display()),
            Err(e) => eprintln!("Manifest save failed: {e}"),
        }
    }

    // ── Storefront API ────────────────────────────────────────────────────────
    if let Some(port) = cli.serve {
        use std::sync::{Arc, Mutex};
        use storefront::store::Storefront as SF;

        println!("Starting storefront API on port {port}…");

        let mut sf = SF::new();
        // Seed with capsules already loaded
        for cap in &capsules {
            if cap.remixable {
                sf.list_blueprint(cap, 0, format!("Auto-listed: {}", cap.persona));
            }
        }
        // Seed a tome from the first valid scroll title
        if let Some(scroll) = scroll_reg.scrolls().iter().find(|s| !s.is_expired()) {
            sf.publish_tome(&scroll.title, &scroll.bound_persona, &scroll.title);
        }

        let app_state = storefront::api::AppState {
            store: Arc::new(Mutex::new(sf)),
            users: Arc::new(Mutex::new(storefront::UserStore::default())),
            vault_dir: cli.vault.clone(),
        };
        let app = storefront::api::router(app_state);
        let addr = format!("0.0.0.0:{port}");

        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
            println!("Storefront listening on http://{addr}");
            println!("  GET /blueprints  GET /collections  GET /tomes  GET /status");
            axum::serve(listener, app).await.unwrap();
        });
        return Ok(());
    }

    if cli.headless {
        println!("Headless boot complete. All systems nominal.");
        return Ok(());
    }

    // ── egui UI ───────────────────────────────────────────────────────────────
    #[cfg(feature = "ui")]
    {
        use storefront::store::Storefront as SF;
        use theater::control_room::ControlRoomApp;
        use user_profile::{CollectionManager, ProfileManager};

        let persona_list = personas::canonical_personas();
        let plugin_slots: Vec<(String, String, bool)> = core
            .plugins.values()
            .map(|s| (s.id.clone(), s.name.clone(), s.enabled))
            .collect();
        let persona_data: Vec<(String, Option<mythos::faction::Faction>, f32)> = persona_list
            .iter()
            .map(|p| (p.name.clone(), p.faction, p.resonance_hz()))
            .collect();
        let remix_links: Vec<(String, String, String)> = remix_lineage
            .links.iter()
            .map(|l| (l.source_capsule_id.clone(), l.derived_capsule_id.clone(), l.lineage_hash.clone()))
            .collect();

        // Profile + collections
        let profile_mgr = ProfileManager::load(&cli.vault)
            .unwrap_or_else(|e| { eprintln!("profile load: {e}"); ProfileManager { profile: Default::default(), path: cli.vault.join(".vaultforge/profile.json") } });
        let col_mgr = CollectionManager::load(&cli.vault)
            .unwrap_or_else(|e| { eprintln!("collections load: {e}"); CollectionManager { collections: vec![], path: cli.vault.join(".vaultforge/collections.json") } });

        // Seed storefront blueprints for the UI (no server needed)
        let mut sf = SF::new();
        for cap in &capsules {
            sf.list_blueprint(cap, if cap.tier == mythos::capsule::VaultTier::Free { 0 } else { 50 }, format!("Auto-listed: {}", cap.persona));
        }
        let blueprints: Vec<_> = sf.all_blueprints().into_iter().cloned().collect();
        let sf_collections: Vec<_> = sf.collections.values().cloned().collect();

        let app = ControlRoomApp::new(
            theme, capsules, persona_data, plugin_slots, remix_links,
            profile_mgr.profile, col_mgr.collections,
            blueprints, sf_collections, cli.vault.clone(),
        );

        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("VaultForge · Control Room")
                .with_inner_size([920.0, 680.0]),
            ..Default::default()
        };
        eframe::run_native("VaultForge", options, Box::new(|_cc| Ok(Box::new(app))))
            .map_err(|e| anyhow::anyhow!("eframe: {e}"))?;
    }

    #[cfg(not(feature = "ui"))]
    {
        println!("UI not compiled. Run with --features ui, or pass --headless.");
    }

    Ok(())
}
