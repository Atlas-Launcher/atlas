import { EventEmitter } from "events";

interface WhitelistEmitter extends EventEmitter {
  emit(event: "whitelist:update", payload: WhitelistUpdate): boolean;
  on(event: "whitelist:update", listener: (payload: WhitelistUpdate) => void): this;
  off(event: "whitelist:update", listener: (payload: WhitelistUpdate) => void): this;
}

export type WhitelistUpdate = {
  packId: string;
  source?: string;
};

declare global {
  // eslint-disable-next-line no-var
  var __atlasWhitelistEvents: WhitelistEmitter | undefined;
}

const emitter: WhitelistEmitter = globalThis.__atlasWhitelistEvents ?? new EventEmitter();
if (!globalThis.__atlasWhitelistEvents) {
  emitter.setMaxListeners(100);
  globalThis.__atlasWhitelistEvents = emitter;
}

export function emitWhitelistUpdate(update: WhitelistUpdate) {
  emitter.emit("whitelist:update", update);
}

export function onWhitelistUpdate(handler: (update: WhitelistUpdate) => void) {
  emitter.on("whitelist:update", handler);
  return () => emitter.off("whitelist:update", handler);
}
