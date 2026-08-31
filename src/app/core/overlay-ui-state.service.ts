import { Service, signal } from "@angular/core";

/** Window-chrome state driven by the tray's native menu and the overlay's own custom menu (see adapter::menu in src-tauri). */
@Service()
export class OverlayUiStateService {
  private readonly lockedSignal = signal(false);
  // Matches tauri.conf.json's initial `alwaysOnTop: true` for the main window.
  private readonly alwaysOnTopSignal = signal(true);

  readonly locked = this.lockedSignal.asReadonly();
  readonly alwaysOnTop = this.alwaysOnTopSignal.asReadonly();

  setLocked(value: boolean): void {
    this.lockedSignal.set(value);
  }

  setAlwaysOnTop(value: boolean): void {
    this.alwaysOnTopSignal.set(value);
  }
}
