import { notFound, redirect } from "next/navigation";

import { PERSONAS, type PersonaId } from "@/lib/docs/types";

type ReaderPageProps = {
  params: Promise<{ persona: string; slug: string[] }>;
};

export default async function ReaderPage({ params }: ReaderPageProps) {
  const { persona, slug } = await params;

  if (!PERSONAS.includes(persona as PersonaId)) {
    notFound();
  }

  const personaId = persona as PersonaId;
  const nextPath = slug.length > 0 ? `/docs/${personaId}/${slug.join("/")}` : `/docs/${personaId}`;
  redirect(nextPath);
}
