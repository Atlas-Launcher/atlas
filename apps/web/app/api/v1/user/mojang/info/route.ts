import { NextResponse } from "next/server";
import { getAuthenticatedUserId } from "@/lib/auth/request-user";
import { db } from "@/lib/db";
import { users } from "@/lib/db/schema";
import { eq } from "drizzle-orm";

export async function GET(request: Request) {
    const userId = await getAuthenticatedUserId(request);
    if (!userId) {
        return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
    }

    const [user] = await db
        .select({
            mojangUsername: users.mojangUsername,
            mojangUuid: users.mojangUuid,
        })
        .from(users)
        .where(eq(users.id, userId))
        .limit(1);

    if (!user) {
        return NextResponse.json({ error: "User not found" }, { status: 404 });
    }

    return NextResponse.json({
        username: user.mojangUsername,
        uuid: user.mojangUuid,
    });
}
