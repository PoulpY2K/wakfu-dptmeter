import { describe, expect, it } from "vitest";
import { FightEvent } from "./fight-event.model";
import { FightStoreService } from "./fight-store.service";
import { participantDpt } from "./participant.model";

describe("FightStoreService", () => {
  it("starts idle with no participants", () => {
    const store = new FightStoreService();

    expect(store.status()).toBe("idle");
    expect(store.fightId()).toBeNull();
    expect(store.participants()).toEqual([]);
  });

  it("folds a short fight into per-character totals", () => {
    const store = new FightStoreService();

    const events: FightEvent[] = [
      { FightStarted: { fight_id: 1568151141 } },
      {
        CharacterIdentified: {
          fight_id: 1568151141,
          name: "Soeur Zerker",
          entity_id: -1724034221200073,
          side: "Enemy",
        },
      },
      {
        CharacterIdentified: {
          fight_id: 1568151141,
          name: "Blampy",
          entity_id: 5547447,
          side: "Player",
        },
      },
      {
        TurnStarted: {
          fight_id: 1568151141,
          name: "Blampy",
          entity_id: 5547447,
          side: "Player",
          turn_number: 1,
        },
      },
      // Rust has already resolved this to the owning player's name, even if
      // it was really cast by one of Blampy's summons — the store never
      // needs to know that.
      {
        ActionRecorded: {
          fight_id: 1568151141,
          source: "Blampy",
          target: "Soeur Zerker",
          amount: -1500,
          kind: "Damage",
          element: "Feu",
          spell_name: "Explosion",
          is_critical: true,
          turn_number: 1,
        },
      },
      {
        ActionRecorded: {
          fight_id: 1568151141,
          source: "Blampy",
          target: "Blampy",
          amount: 400,
          kind: "Heal",
          element: null,
          spell_name: "Mot de soin",
          is_critical: false,
          turn_number: 1,
        },
      },
      { FightEnded: { fight_id: 1568151141 } },
    ];

    for (const event of events) {
      store.process(event);
    }

    expect(store.status()).toBe("ended");
    expect(store.fightId()).toBe(1568151141);

    const [blampy, soeurZerker] = store.participants();
    expect(blampy).toMatchObject({
      name: "Blampy",
      side: "Player",
      damageDealt: 1500,
      healingDone: 400,
      damageTaken: 0,
      turnsTaken: 1,
      criticalHits: 1,
    });
    expect(participantDpt(blampy)).toBe(1500);

    expect(soeurZerker).toMatchObject({
      name: "Soeur Zerker",
      side: "Enemy",
      damageDealt: 0,
      damageTaken: 1500,
      turnsTaken: 0,
    });
  });

  it("clears previous participants when a new fight starts", () => {
    const store = new FightStoreService();
    store.process({ FightStarted: { fight_id: 1 } });
    store.process({
      CharacterIdentified: { fight_id: 1, name: "Blampy", entity_id: 1, side: "Player" },
    });
    store.process({ FightEnded: { fight_id: 1 } });

    store.process({ FightStarted: { fight_id: 2 } });

    expect(store.status()).toBe("active");
    expect(store.fightId()).toBe(2);
    expect(store.participants()).toEqual([]);
  });

  it("resetToIdle clears state back to its initial shape", () => {
    const store = new FightStoreService();
    store.process({ FightStarted: { fight_id: 1 } });
    store.process({
      CharacterIdentified: { fight_id: 1, name: "Blampy", entity_id: 1, side: "Player" },
    });

    store.resetToIdle();

    expect(store.status()).toBe("idle");
    expect(store.fightId()).toBeNull();
    expect(store.participants()).toEqual([]);
  });
});
