"use client";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

interface AccountTabProps {
  onAddPasskey: () => void;
  onOpenDeviceFlow: () => void;
}

export default function AccountTab({ onAddPasskey, onOpenDeviceFlow }: AccountTabProps) {
  return (
    <div className="grid gap-6 lg:grid-cols-2">
      <Card>
        <CardHeader>
          <CardTitle>Passkeys</CardTitle>
          <CardDescription>
            Register a hardware-backed passkey for quick sign-in.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Button onClick={onAddPasskey}>Add Passkey</Button>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Device Login</CardTitle>
          <CardDescription>
            Enter a launcher device code to authorize the session.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Button variant="outline" onClick={onOpenDeviceFlow}>
            Open Device Flow
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
