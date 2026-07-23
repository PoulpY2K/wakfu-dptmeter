import { Service, computed, signal } from "@angular/core";
import { ActionRecordedPayload, FightEvent } from "./fight-event.model";
import { ParticipantStats, createEmptyParticipant } from "./participant.model";

export type FightStatus = "idle" | "active" | "ended";

/**
 * Folds the raw `FightEvent` stream into per-character totals for display.
 * This is a plain reduce over data Rust already parsed and attributed (sum
 * amounts, bucket by name) — no log parsing, no turn/attribution inference.
 */
@Service()
export class FightStoreService {
  private readonly participantsByName = signal(new Map<string, ParticipantStats>());
  private readonly fightIdSignal = signal<number | null>(null);
  private readonly statusSignal = signal<FightStatus>("idle");

  readonly fightId = this.fightIdSignal.asReadonly();
  readonly status = this.statusSignal.asReadonly();
  readonly participants = computed(() =>
    [...this.participantsByName().values()].sort((a, b) => b.damageDealt - a.damageDealt),
  );

  /** Manual reset (from the "Reset current fight" menu action), independent of any log event. */
  resetToIdle(): void {
    this.fightIdSignal.set(null);
    this.statusSignal.set("idle");
    this.participantsByName.set(new Map());
  }

  process(event: FightEvent): void {
    if ("FightStarted" in event) {
      this.fightIdSignal.set(event.FightStarted.fight_id);
      this.statusSignal.set("active");
      this.participantsByName.set(new Map());
      return;
    }
    if ("CharacterIdentified" in event) {
      const { name, side } = event.CharacterIdentified;
      this.upsert(name, (participant) => ({ ...participant, side }));
      return;
    }
    if ("TurnStarted" in event) {
      const { name, turn_number } = event.TurnStarted;
      this.upsert(name, (participant) => ({ ...participant, turnsTaken: turn_number }));
      return;
    }
    if ("ActionRecorded" in event) {
      this.applyAction(event.ActionRecorded);
      return;
    }
    if ("FightEnded" in event) {
      this.statusSignal.set("ended");
    }
  }

  private applyAction(action: ActionRecordedPayload): void {
    const magnitude = Math.abs(action.amount);
    if (action.kind === "Damage") {
      this.upsert(action.source, (participant) => ({
        ...participant,
        damageDealt: participant.damageDealt + magnitude,
        criticalHits: participant.criticalHits + (action.is_critical ? 1 : 0),
      }));
      this.upsert(action.target, (participant) => ({
        ...participant,
        damageTaken: participant.damageTaken + magnitude,
      }));
      return;
    }
    this.upsert(action.source, (participant) => ({
      ...participant,
      healingDone: participant.healingDone + magnitude,
    }));
  }

  private upsert(
    name: string,
    updateFn: (participant: ParticipantStats) => ParticipantStats,
  ): void {
    this.participantsByName.update((current) => {
      const next = new Map(current);
      const existing = next.get(name) ?? createEmptyParticipant(name);
      next.set(name, updateFn(existing));
      return next;
    });
  }
}
