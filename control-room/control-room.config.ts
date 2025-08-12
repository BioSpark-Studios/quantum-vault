import vaultwarden from '../capsules/vaultwarden.capsule.md';
import licenseGate from '../capsules/license-gate.capsule.md';
import remixChain from '../capsules/remix-chain.capsule.md';

import vaultAccessScroll from '../scrolls/vault-access.remix.md';
import licenseGateScroll from '../scrolls/license-gate.remix.md';
import remixChainScroll from '../scrolls/remix-chain.remix.md';

export default {
  capsules: [
    {
      id: 'vaultwarden',
      glyph: '🛡️🔒🜁',
      persona: 'Sentinel',
      scroll: 'vault-access.remix.md',
      pluginSlot: 'LicenseGate',
      traits: ['Sovereign', 'Bound', 'Sentinel'],
      capsule: vaultwarden,
    },
    {
      id: 'licenseGate',
      glyph: '🗝️📜🜂',
      persona: 'Gatekeeper',
      scroll: 'license-gate.remix.md',
      pluginSlot: 'LicenseCheck',
      traits: ['Lineage', 'Tiered', 'Verifier'],
      capsule: licenseGate,
    },
    {
      id: 'remixChain',
      glyph: '🔗🌀🜃',
      persona: 'Echo Weaver',
      scroll: 'remix-chain.remix.md',
      pluginSlot: 'LineageTracker',
      traits: ['Echo', 'Ancestral', 'Threader'],
      capsule: remixChain,
    },
  ],
  scrolls: {
    'vault-access.remix.md': vaultAccessScroll,
    'license-gate.remix.md': licenseGateScroll,
    'remix-chain.remix.md': remixChainScroll,
  },
  pluginSlots: ['LicenseGate', 'LicenseCheck', 'LineageTracker'],
};
