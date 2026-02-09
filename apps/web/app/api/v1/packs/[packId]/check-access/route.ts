import { NextResponse } from "next/server";
import { and, eq } from "drizzle-orm";
import { db } from "@/lib/db";
import { packMembers } from "@/lib/db/schema";
import { getAuthenticatedUserId } from "@/lib/auth/request-user";
import { getAuthenticatedRunnerPackId } from "@/lib/auth/runner-tokens";
import { users } from "@/lib/db/schema";

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
        return NextResponse.json({ allowed: true, role: "runner" });
    }

    // Get user role from global users table first (for admin check)
    const [user] = await db
        .select({ role: users.role })
        .from(users)
        .where(eq(users.id, userId))
        .limit(1);

    if (user?.role === "admin") {
        return NextResponse.json({ allowed: true, role: "admin" });
    }

    // Check pack-specific role
    const [membership] = await db
        .select({ role: packMembers.role })
        .from(packMembers)
        .where(
            and(
                eq(packMembers.packId, packId),
                eq(packMembers.userId, userId)
            )
        )
        .limit(1);

    if (!membership) {
        return NextResponse.json({ allowed: false, role: null }, { status: 403 });
    }

    const allowed = membership.role === "creator" || membership.role === "admin";
    return NextResponse.json({ allowed, role: membership.role });
}
