import { Component, ElementRef, afterNextRender, inject, input, output, signal } from "@angular/core";
import { invoke } from "@tauri-apps/api/core";
import { FightStoreService } from "../core/fight-store.service";
import { OverlayUiStateService } from "../core/overlay-ui-state.service";

@Component({
  selector: "app-context-menu",
  templateUrl: "./context-menu.component.html",
  styleUrl: "./context-menu.component.css",
  host: {
    "(document:keydown.escape)": "closed.emit()",
  },
})
export class ContextMenuComponent {
  readonly x = input.required<number>();
  readonly y = input.required<number>();
  readonly closed = output<void>();

  protected readonly fightStore = inject(FightStoreService);
  protected readonly overlayUiState = inject(OverlayUiStateService);

  protected readonly left = signal(0);
  protected readonly top = signal(0);
  protected readonly positioned = signal(false);

  // `inject<T>()`'s explicit type argument is required here: the ProviderToken<T>
  // union overload fails to infer T from an `ElementRef<HTMLElement>` instantiation
  // expression passed positionally, silently widening `nativeElement` to `any`.
  private readonly elementRef = inject<ElementRef<HTMLElement>>(ElementRef);

  constructor() {
    // Clamp the menu inside the viewport — the overlay window is only
    // 320x400, so a click near an edge must not let the menu spill outside.
    afterNextRender(() => {
      const menu = this.elementRef.nativeElement.querySelector<HTMLElement>(".menu");
      if (!menu) {
        return;
      }
      const maxLeft = Math.max(0, window.innerWidth - menu.offsetWidth);
      const maxTop = Math.max(0, window.innerHeight - menu.offsetHeight);
      this.left.set(Math.min(this.x(), maxLeft));
      this.top.set(Math.min(this.y(), maxTop));
      this.positioned.set(true);
    });
  }

  protected resetFight(): void {
    this.fightStore.resetToIdle();
    this.closed.emit();
  }

  protected toggleLock(): void {
    void invoke("toggle_lock").catch((err) => console.error("failed to toggle lock:", err));
    this.closed.emit();
  }

  protected toggleAlwaysOnTop(): void {
    void invoke("toggle_always_on_top").catch((err) =>
      console.error("failed to toggle always-on-top:", err),
    );
    this.closed.emit();
  }

  protected quit(): void {
    void invoke("quit_app").catch((err) => console.error("failed to quit:", err));
  }
}
