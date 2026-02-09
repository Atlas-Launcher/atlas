import { NextResponse } from "next/server";
import { and, eq } from "drizzle-orm";

import { db } from "@/lib/db";
import { packMembers } from "@/lib/db/schema";
import { getAuthenticatedUserId } from "@/lib/auth/request-user";
import { getAuthenticatedRunnerPackId } from "@/lib/auth/runner-tokens";
import { onWhitelistUpdate } from "@/lib/whitelist-events";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

interface RouteParams {
  params: Promise<{
    packId: string;
  }>;
}

export async function GET(request: Request, { params }: RouteParams) {
  const { packId } = await params;
  const userId = await getAuthenticatedUserId(request);
  if (!userId) {
    const runnerPackId = await getAuthenticatedRunnerPackId(request);
    if (!runnerPackId) {
      return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
    }
    if (runnerPackId !== packId) {
      return NextResponse.json({ error: "Forbidden" }, { status: 403 });
    }
  } else {
    const [membership] = await db
      .select({ role: packMembers.role })
      .from(packMembers)
      .where(and(eq(packMembers.packId, packId), eq(packMembers.userId, userId)))
      .limit(1);

    if (!membership || (membership.role !== "creator" && membership.role !== "admin")) {
      return NextResponse.json({ error: "Forbidden" }, { status: 403 });
    }
  }

  const encoder = new TextEncoder();
  const stream = new ReadableStream<Uint8Array>({
    start(controller) {
      const send = (payload: Record<string, string>) => {
        controller.enqueue(
          encoder.encode(`event: whitelist\ndata: ${JSON.stringify(payload)}\n\n`)
        );
      };

      send({ type: "ready", packId });

      const unsubscribe = onWhitelistUpdate((update) => {
        if (update.packId !== packId) {
          return;
        }
        send({
          type: "whitelist",
          packId,
          source: update.source ?? "hub",
          at: new Date().toISOString(),
        });
      });

      const keepAlive = setInterval(() => {
        controller.enqueue(encoder.encode(": keep-alive\n\n"));
      }, 15000);

      const close = () => {
        clearInterval(keepAlive);
        unsubscribe();
        controller.close();
      };

      request.signal.addEventListener("abort", close);
    },
  });

  return new Response(stream, {
    headers: {
      "Content-Type": "text/event-stream; charset=utf-8",
      "Cache-Control": "no-cache, no-transform",
      Connection: "keep-alive",
    },
  });
}
