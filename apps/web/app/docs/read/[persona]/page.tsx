import { notFound, redirect } from "next/navigation";

import { PERSONAS, type PersonaId } from "@/lib/docs/types";

type ReaderPersonaPageProps = {
  params: Promise<{ persona: string }>;
};

export default async function ReaderPersonaPage({ params }: ReaderPersonaPageProps) {
  const { persona } = await params;

  if (!PERSONAS.includes(persona as PersonaId)) {
    notFound();
  }

  redirect(`/docs/${persona as PersonaId}`);
}
