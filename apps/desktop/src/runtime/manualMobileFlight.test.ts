import { describe, expect, it, vi } from "vitest";
import { ManualMobileFlight } from "./manualMobileFlight";

describe("manual mobile command admission", () => {
  it("admits one Observe, ignores overlap before IPC, and admits a later Observe", async () => {
    const states: boolean[] = [];
    const flight = new ManualMobileFlight(value => states.push(value));
    let finish!: (value: string) => void;
    const invoke = vi.fn(() => new Promise<string>(resolve => { finish = resolve; }));
    const first = flight.run(invoke);
    expect(flight.busy).toBe(true);
    expect(await flight.run(invoke)).toBeUndefined();
    expect(invoke).toHaveBeenCalledTimes(1);
    finish("completed-observation");
    expect(await first).toBe("completed-observation");
    expect(flight.busy).toBe(false);
    expect(await flight.run(async () => "next-observation")).toBe("next-observation");
    expect(states).toEqual([true, false, true, false]);
  });

  it("releases admission after IPC rejection without fabricating success", async () => {
    const flight = new ManualMobileFlight(() => {});
    await expect(flight.run(async () => { throw new Error("DEVICE_DISCONNECTED"); })).rejects.toThrow("DEVICE_DISCONNECTED");
    expect(flight.busy).toBe(false);
    expect(await flight.run(async () => "recovered")).toBe("recovered");
  });

  it("preserves a backend busy result and releases admission", async () => {
    const flight = new ManualMobileFlight(() => {});
    const rejected = { status: "failed", errorCode: "MOBILE_BUSY" };
    expect(await flight.run(async () => rejected)).toBe(rejected);
    expect(flight.busy).toBe(false);
  });
});
