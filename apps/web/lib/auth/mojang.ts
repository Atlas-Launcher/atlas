import { db } from "@/lib/db";
import { accounts, users } from "@/lib/db/schema";
import { eq, and } from "drizzle-orm";

export async function syncMojangProfile(userId: string) {
    // 1. Get Microsoft account
    const [account] = await db
        .select()
        .from(accounts)
        .where(and(eq(accounts.userId, userId), eq(accounts.providerId, "microsoft")))
        .limit(1);

    if (!account || !account.accessToken) {
        throw new Error("No linked Microsoft account found");
    }

    try {
        // 2. Xbox Live Auth
        const xblResponse = await fetch("https://user.auth.xboxlive.com/user/authenticate", {
            method: "POST",
            headers: { "Content-Type": "application/json", Accept: "application/json" },
            body: JSON.stringify({
                Properties: {
                    AuthMethod: "RPS",
                    SiteName: "user.auth.xboxlive.com",
                    RpsTicket: `d=${account.accessToken}`,
                },
                RelyingParty: "http://auth.xboxlive.com",
                TokenType: "JWT",
            }),
        });
        const xblData = await xblResponse.json();
        const xblToken = xblData.Token;
        const uhs = xblData.DisplayClaims.xui[0].uhs;

        // 3. XSTS Auth
        const xstsResponse = await fetch("https://xsts.auth.xboxlive.com/xsts/authorize", {
            method: "POST",
            headers: { "Content-Type": "application/json", Accept: "application/json" },
            body: JSON.stringify({
                Properties: { SandboxId: "RETAIL", UserTokens: [xblToken] },
                RelyingParty: "rp://api.minecraftservices.com/",
                TokenType: "JWT",
            }),
        });
        const xstsData = await xstsResponse.json();
        const xstsToken = xstsData.Token;

        // 4. Minecraft Auth
        const mcResponse = await fetch("https://api.minecraftservices.com/authentication/login_with_xbox", {
            method: "POST",
            headers: { "Content-Type": "application/json", Accept: "application/json" },
            body: JSON.stringify({ identityToken: `XBL3.0 x=${uhs};${xstsToken}` }),
        });
        const mcData = await mcResponse.json();
        const mcToken = mcData.access_token;

        // 5. Get Minecraft Profile
        const profileResponse = await fetch("https://api.minecraftservices.com/minecraft/profile", {
            headers: { Authorization: `Bearer ${mcToken}` },
        });
        const profileData = await profileResponse.json();

        if (profileData.error) {
            throw new Error(`Minecraft profile error: ${profileData.errorMessage}`);
        }

        // 6. Update User record
        await db.update(users)
            .set({
                mojangUsername: profileData.name,
                mojangUuid: profileData.id,
            })
            .where(eq(users.id, userId));

        return { username: profileData.name, uuid: profileData.id };
    } catch (error) {
        console.error("Failed to sync Mojang profile:", error);
        throw error;
    }
}
