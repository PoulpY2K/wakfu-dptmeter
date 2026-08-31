import { Side } from "./fight-event.model";

export interface ParticipantStats {
  name: string;
  side: Side;
  damageDealt: number;
  healingDone: number;
  damageTaken: number;
  turnsTaken: number;
  criticalHits: number;
}

export function createEmptyParticipant(name: string): ParticipantStats {
  return {
    name,
    // Placeholder only: CharacterIdentified always arrives before any
    // ActionRecorded/TurnStarted referencing this name, so this default
    // is overwritten before it's ever shown.
    side: "Player",
    damageDealt: 0,
    healingDone: 0,
    damageTaken: 0,
    turnsTaken: 0,
    criticalHits: 0,
  };
}

export function participantDpt(stats: ParticipantStats): number {
  return stats.damageDealt / Math.max(stats.turnsTaken, 1);
}
