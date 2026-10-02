/** Shared by manual Observe and navigation; admission closes before the first await. */
export class ManualMobileFlight {
  busy = false;

  constructor(private readonly onBusyChange: (busy: boolean) => void) {}

  async run<T>(operation: () => Promise<T>): Promise<T | undefined> {
    if (this.busy) return;
    this.busy = true;
    this.onBusyChange(true);
    try { return await operation(); }
    finally { this.busy = false; this.onBusyChange(false); }
  }
}
