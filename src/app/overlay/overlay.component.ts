import { Component, computed, inject, signal } from "@angular/core";
import { FightStoreService } from "../core/fight-store.service";
import { OverlayUiStateService } from "../core/overlay-ui-state.service";
import { ContextMenuComponent } from "./context-menu.component";
import { ParticipantRowComponent } from "./participant-row.component";

interface ContextMenuPosition {
  x: number;
  y: number;
}

@Component({
  selector: "app-overlay",
  imports: [ParticipantRowComponent, ContextMenuComponent],
  templateUrl: "./overlay.component.html",
  styleUrl: "./overlay.component.css",
  host: {
    "(document:contextmenu)": "onContextMenu($event)",
  },
})
export class OverlayComponent {
  protected readonly fightStore = inject(FightStoreService);
  protected readonly overlayUiState = inject(OverlayUiStateService);

  protected readonly topDamage = computed(() =>
    Math.max(1, ...this.fightStore.participants().map((participant) => participant.damageDealt)),
  );

  protected readonly contextMenuPosition = signal<ContextMenuPosition | null>(null);

  protected onContextMenu(event: MouseEvent): void {
    event.preventDefault();
    this.contextMenuPosition.set({ x: event.clientX, y: event.clientY });
  }

  protected closeContextMenu(): void {
    this.contextMenuPosition.set(null);
  }
}
