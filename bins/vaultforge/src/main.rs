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
    println!("╚══════════════════════════════════════╝");
    println!();

    // ── Boot Genesis Container ───────────────────────────────────────────────
    let genesis = mythos::genesis::GenesisContainer::new(
        myth_wire::new_id(),
        "Quantum Vault",
        "biospark-studios",
    );
    println!("Genesis:  {} ({})", genesis.name, genesis.id);
    println!("BDna:     {}…", &genesis.bdna_signature[..16]);
    println!("Resonance: {:.1} Hz", genesis.resonance_hz);
    println!();

    // ── Load Scrolls ─────────────────────────────────────────────────────────
    let scroll_reg = vault_scrolls::ScrollRegistry::load(&cli.vault)?;
    println!("Scrolls loaded: {}", scroll_reg.scrolls().len());
    for scroll in scroll_reg.scrolls() {
        let status = if scroll.is_expired() { "EXPIRED" } else { "valid" };
        println!(
            "  [{:7}] {} (tier: {:?}, remixable: {})",
            status, scroll.id, scroll.tier, scroll.remixable
        );
    }
    println!();

    // ── Load Capsules ─────────────────────────────────────────────────────────
    let capsules = qgcp::load_capsules(&cli.vault)?;
    println!("Capsules loaded: {}", capsules.len());
    for cap in &capsules {
        println!(
            "  {} — persona: {}, tier: {:?}, remixable: {}",
            cap.id, cap.persona, cap.tier, cap.remixable
        );
    }
    println!();

    // ── Load Glyphs ───────────────────────────────────────────────────────────
    let glyph_reg = vault_glyphs::GlyphRegistry::load(&cli.vault)?;
    println!("Glyphs loaded: {}", glyph_reg.glyphs().len());
    println!();

    // ── Boot Personas ─────────────────────────────────────────────────────────
    let persona_list = personas::canonical_personas();
    println!("Personas: {}", persona_list.len());
    for p in &persona_list {
        let faction = p.faction.map(|f| f.to_string()).unwrap_or_else(|| "—".to_string());
        println!("  {} (faction: {}, resonance: {:.1} Hz)", p.name, faction, p.resonance_hz());
    }
    println!();

    // ── Boot Vault Core ───────────────────────────────────────────────────────
    let mut core = vault_core::VaultCore::new(&genesis.id);
    for slot in vault_core::default_plugin_slots() {
        core.register_plugin(slot);
    }
    println!("Plugin slots: {}", core.plugins.len());
    for (id, slot) in &core.plugins {
        println!("  [{}] {} (enabled: {})", id, slot.name, slot.enabled);
    }
    println!();

    // ── Apply Theme ───────────────────────────────────────────────────────────
    let faction = cli.theme
        .as_deref()
        .and_then(Faction::from_str)
        .unwrap_or(Faction::Sylvanid);
    let theme = theater::VaultTheme::from_faction(faction);
    println!(
        "Theme: {:?} — primary: {}, background: {}",
        theme.faction, theme.palette.primary, theme.palette.background
    );
    println!();

    if cli.headless {
        println!("Running headless. Topology summary complete.");
    } else {
        println!("UI rendering not yet wired (pass --headless to suppress this message).");
    }

    Ok(())
}
