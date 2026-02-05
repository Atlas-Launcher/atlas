"use client";

import Link from "next/link";
import { ReactNode } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

interface PackHeaderProps {
  name?: string | null;
  slug?: string | null;
  packId: string;
  actions?: ReactNode;
}

export default function PackHeader({ name, slug, packId, actions }: PackHeaderProps) {
  return (
    <Card>
      <CardHeader>
        <Badge variant="secondary">Pack</Badge>
        <CardTitle>{name ?? "Pack Dashboard"}</CardTitle>
        <CardDescription>{slug ?? packId}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex gap-2">
          <Badge variant="outline">ID: {packId}</Badge>
          {slug ? <Badge>{slug}</Badge> : null}
        </div>
        <div className="flex items-center gap-3">
          <Link href="/dashboard">
            <Button variant="outline">All Packs</Button>
          </Link>
          {actions}
        </div>
      </CardContent>
    </Card>
  );
}
