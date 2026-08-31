import { Component, inject } from "@angular/core";
import { RouterOutlet } from "@angular/router";
import { FightEventBridgeService } from "./core/fight-event-bridge.service";

@Component({
  selector: "app-root",
  imports: [RouterOutlet],
  templateUrl: "./app.component.html",
})
export class AppComponent {
  private readonly fightEventBridge = inject(FightEventBridgeService);

  constructor() {
    void this.fightEventBridge.start().catch((err) =>
      console.error("failed to start fight event bridge:", err),
    );
  }
}
