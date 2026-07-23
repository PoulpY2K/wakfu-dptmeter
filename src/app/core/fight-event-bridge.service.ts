import { Service, inject } from "@angular/core";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { FightEvent } from "./fight-event.model";
import { FightStoreService } from "./fight-store.service";
import { OverlayUiStateService } from "./overlay-ui-state.service";

/**
 * Forwards Tauri IPC events into frontend state: `fight-event` payloads into
 * `FightStoreService`, and the native menu's `overlay-*` events (see
 * adapter::menu in src-tauri) into the relevant store/service.
 */
@Service()
export class FightEventBridgeService {
  private readonly fightStore = inject(FightStoreService);
  private readonly overlayUiState = inject(OverlayUiStateService);
  private unlistenFns: UnlistenFn[] | null = null;

  async start(): Promise<void> {
    if (this.unlistenFns) {
      return;
    }
    try {
      const state = await invoke<{ locked: boolean; always_on_top: boolean }>("get_overlay_state");
      this.overlayUiState.setLocked(state.locked);
      this.overlayUiState.setAlwaysOnTop(state.always_on_top);
    } catch (err) {
      console.error("failed to hydrate overlay UI state:", err);
    }
    this.unlistenFns = await Promise.all([
      listen<FightEvent>("fight-event", (event) => {
        this.fightStore.process(event.payload);
      }),
      listen("overlay-reset-fight", () => {
        this.fightStore.resetToIdle();
      }),
      listen<boolean>("overlay-locked-changed", (event) => {
        this.overlayUiState.setLocked(event.payload);
      }),
      listen<boolean>("overlay-always-on-top-changed", (event) => {
        this.overlayUiState.setAlwaysOnTop(event.payload);
      }),
    ]);
  }
}
