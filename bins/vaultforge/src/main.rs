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

    /// Run without UI (headless mode)
    #[arg(long)]
    headless: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    println!("╔══════════════════════════════════════╗");
    println!("║     VaultForge  ·  myth-os  v0.1     ║");
    println!("╚══════════════════════════════════════╝\n");

    // ── Vaultmap ──────────────────────────────────────────────────────────────
    match qgcp::VaultMap::load(&cli.vault) {
        Ok(vm) => {
            let inner = vm.inner();
            println!("Vault: {} — {}", inner.repo, inner.description);
            println!("Tiers: {}", inner.sovereign_tiers.join(" · "));
            println!("Branches: {}", inner.branches.iter().map(|b| b.name.as_str()).collect::<Vec<_>>().join(", "));
        }
        Err(e) => println!("(vaultmap skipped: {})", e),
    }
    println!();

    // ── Genesis Container ─────────────────────────────────────────────────────
    let genesis = mythos::genesis::GenesisContainer::new(
        myth_wire::new_id(),
        "Quantum Vault",
        "biospark-studios",
    );
    println!("Genesis:   {} ({})", genesis.name, genesis.id);
    println!("BDna:      {}…", &genesis.bdna_signature[..16]);
    println!("Resonance: {:.1} Hz\n", genesis.resonance_hz);

    // ── Scrolls ───────────────────────────────────────────────────────────────
    let scroll_reg = vault_scrolls::ScrollRegistry::load(&cli.vault)?;
    println!("Scrolls loaded: {}", scroll_reg.scrolls().len());
    for scroll in scroll_reg.scrolls() {
        let status = if scroll.is_expired() { "EXPIRED" } else { "valid  " };
        println!(
            "  [{status}] {} — {:?} · remixable: {} · persona: {}",
            scroll.id, scroll.tier, scroll.remixable, scroll.bound_persona
        );
    }
    println!();

    // ── Capsules ──────────────────────────────────────────────────────────────
    let capsules = qgcp::load_capsules(&cli.vault)?;
    println!("Capsules loaded: {}", capsules.len());
    for cap in &capsules {
        println!(
            "  {} — {:?} · remixable: {} · bdna-weight: {}",
            cap.id, cap.tier, cap.remixable, cap.bdna.weight()
        );
    }
    println!();

    // ── Glyphs ────────────────────────────────────────────────────────────────
    let glyph_reg = vault_glyphs::GlyphRegistry::load(&cli.vault)?;
    println!("Glyphs loaded: {}\n", glyph_reg.glyphs().len());

    // ── Personas ──────────────────────────────────────────────────────────────
    let persona_list = personas::canonical_personas();
    println!("Personas: {}", persona_list.len());
    for p in &persona_list {
        let faction = p.faction.map(|f| format!("{:?}", f)).unwrap_or_else(|| "—".to_string());
        println!("  {} · {} · {:.1}Hz", p.name, faction, p.resonance_hz());
    }
    println!();

    // ── Plugin Slots ──────────────────────────────────────────────────────────
    let mut core = vault_core::VaultCore::new(&genesis.id);
    for slot in vault_core::default_plugin_slots() {
        core.register_plugin(slot);
    }
    println!("Plugin slots: {}", core.plugins.len());
    for (_, slot) in &core.plugins {
        println!("  [{}] {} · enabled: {}", slot.id, slot.name, slot.enabled);
    }
    println!();

    // ── Theme ─────────────────────────────────────────────────────────────────
    let faction = cli.theme.as_deref().and_then(Faction::from_str).unwrap_or(Faction::Sylvanid);
    let theme = theater::VaultTheme::from_faction(faction);
    println!("Theme: {:?} · primary: {} · bg: {}\n", theme.faction, theme.palette.primary, theme.palette.background);

    // ── Remix Engine demo ─────────────────────────────────────────────────────
    let mut remix_lineage = remix_engine::RemixLineage::new();
    let loaded_caps = qgcp::load_capsules(&cli.vault)?;
    // Demo: try chaining two remixable capsules
    let remixable: Vec<_> = loaded_caps.iter().filter(|c| c.remixable).cloned().collect();
    if remixable.len() >= 2 {
        let source = remixable[0].clone();
        let mut derived = remixable[1].clone();
        derived.id = format!("{}-remix", derived.id);
        if let Ok(pkt) = remix_lineage.register(&source, &mut derived, "vaultforge-demo") {
            println!(
                "Remix chain: {} → {} [EVT@{}Hz]",
                source.id, derived.id, pkt.resonance_hz
            );
        }
    }
    println!("Remix links: {}\n", remix_lineage.links.len());

    if cli.headless {
        println!("Headless boot complete. All systems nominal.");
        return Ok(());
    }

    // ── egui UI ───────────────────────────────────────────────────────────────
    #[cfg(feature = "ui")]
    {
        use theater::control_room::ControlRoomApp;

        let plugin_slots: Vec<(String, String, bool)> = core
            .plugins
            .values()
            .map(|s| (s.id.clone(), s.name.clone(), s.enabled))
            .collect();

        let persona_data: Vec<(String, Option<mythos::faction::Faction>, f32)> = persona_list
            .iter()
            .map(|p| (p.name.clone(), p.faction, p.resonance_hz()))
            .collect();

        let remix_links: Vec<(String, String, String)> = remix_lineage
            .links
            .iter()
            .map(|l| (l.source_capsule_id.clone(), l.derived_capsule_id.clone(), l.lineage_hash.clone()))
            .collect();

        let app = ControlRoomApp {
            theme,
            capsules,
            selected_capsule: None,
            persona_list: persona_data,
            plugin_slots,
            remix_links,
            crossfader: 0.5,
            toggled_plugins: Vec::new(),
        };

        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("VaultForge · Control Room")
                .with_inner_size([900.0, 650.0]),
            ..Default::default()
        };

        eframe::run_native("VaultForge", options, Box::new(|_cc| Ok(Box::new(app))))
            .map_err(|e| anyhow::anyhow!("eframe error: {}", e))?;
    }

    #[cfg(not(feature = "ui"))]
    {
        println!("UI not compiled. Run with --features ui to enable the egui Control Room.");
        println!("Pass --headless to suppress this message.");
    }

    Ok(())
}
