import React from 'react';
import CapsuleViewer from './CapsuleViewer';
import PluginSlot from './PluginSlot';
import PersonaOverlay from './PersonaOverlay';
import RemixLineage from './RemixLineage';
import config from './control-room.config';

export default function ControlRoom() {
  const { capsules, scrolls, pluginSlots } = config;

  return (
    <div className="control-room">
      <h1>🧭 Quantum Vault Control Room</h1>
      {capsules.map((capsule) => (
        <div key={capsule.id} className="capsule-block">
          <CapsuleViewer capsule={capsule} />
          <PersonaOverlay persona={capsule.persona} traits={capsule.traits} />
          <PluginSlot slot={capsule.pluginSlot} traits={capsule.traits} />
          <RemixLineage scroll={scrolls[capsule.scroll]} />
        </div>
      ))}
    </div>
  );
}
