import { Component, computed, input } from "@angular/core";
import { ParticipantStats, participantDpt } from "../core/participant.model";

const numberFormat = new Intl.NumberFormat(undefined, {
  notation: "compact",
  maximumFractionDigits: 1,
});

@Component({
  selector: "app-participant-row",
  templateUrl: "./participant-row.component.html",
  styleUrl: "./participant-row.component.css",
})
export class ParticipantRowComponent {
  readonly stats = input.required<ParticipantStats>();
  readonly topDamage = input.required<number>();

  protected readonly barWidthPercent = computed(() =>
    Math.round((this.stats().damageDealt / this.topDamage()) * 100),
  );
  protected readonly dpt = computed(() => participantDpt(this.stats()));

  protected format(value: number): string {
    return numberFormat.format(value);
  }
}
