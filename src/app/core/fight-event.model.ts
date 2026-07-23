/**
 * Mirrors `src-tauri/src/domain/fight/model.rs`'s `FightEvent`, `Side`, and
 * `ActionKind`. Rust derives plain `Serialize` (no `#[serde(tag = ...)]`),
 * so each variant arrives externally tagged, e.g.
 * `{"ActionRecorded": {"fight_id": 1568151141, ...}}`.
 */

export type Side = "Player" | "Enemy";

export type ActionKind = "Damage" | "Heal";

export interface FightStartedPayload {
  fight_id: number;
}

export interface CharacterIdentifiedPayload {
  fight_id: number;
  name: string;
  entity_id: number;
  side: Side;
}

export interface TurnStartedPayload {
  fight_id: number;
  name: string;
  entity_id: number;
  side: Side;
  turn_number: number;
}

export interface ActionRecordedPayload {
  fight_id: number;
  source: string;
  target: string;
  amount: number;
  kind: ActionKind;
  element: string | null;
  spell_name: string | null;
  is_critical: boolean;
  turn_number: number;
}

export interface FightEndedPayload {
  fight_id: number;
}

export type FightEvent =
  | { FightStarted: FightStartedPayload }
  | { CharacterIdentified: CharacterIdentifiedPayload }
  | { TurnStarted: TurnStartedPayload }
  | { ActionRecorded: ActionRecordedPayload }
  | { FightEnded: FightEndedPayload };
