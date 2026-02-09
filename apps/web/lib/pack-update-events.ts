import { EventEmitter } from "events";

interface PackUpdateEmitter extends EventEmitter {
  emit(event: "pack:update", payload: PackUpdate): boolean;
  on(event: "pack:update", listener: (payload: PackUpdate) => void): this;
  off(event: "pack:update", listener: (payload: PackUpdate) => void): this;
}

export type PackUpdate = {
  packId: string;
  channel: string;
  buildId: string;
  source?: string;
};

declare global {
  // eslint-disable-next-line no-var
  var __atlasPackUpdateEvents: PackUpdateEmitter | undefined;
}

const emitter: PackUpdateEmitter =
  globalThis.__atlasPackUpdateEvents ?? new EventEmitter();
if (!globalThis.__atlasPackUpdateEvents) {
  emitter.setMaxListeners(100);
  globalThis.__atlasPackUpdateEvents = emitter;
}

export function emitPackUpdate(update: PackUpdate) {
  emitter.emit("pack:update", update);
}

export function onPackUpdate(handler: (update: PackUpdate) => void) {
  emitter.on("pack:update", handler);
  return () => emitter.off("pack:update", handler);
}