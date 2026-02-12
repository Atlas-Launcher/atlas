export const PERSONAS = ["player", "creator", "host"] as const;

export type PersonaId = (typeof PERSONAS)[number];

export const DOC_INTENTS = ["getting-started", "reference", "troubleshooting", "tutorial"] as const;

export type DocIntent = (typeof DOC_INTENTS)[number];

export type DocFrontmatter = {
  title: string;
  summary: string;
  persona: PersonaId;
  order: number;
  keywords: string[];
  intent: DocIntent;
};

export type DocHeading = {
  level: number;
  text: string;
  id: string;
};

export type DocItem = {
  id: string;
  title: string;
  summary: string;
  persona: PersonaId;
  order: number;
  keywords: string[];
  intent: DocIntent;
  relativePath: string;
  slug: string;
  routePath: string;
  body: string;
  headings: DocHeading[];
};

export type PersonaNavItem = {
  slug: string;
  file: string;
  title: string;
  intent: DocIntent;
};

export type PersonaNavSection = {
  id: PersonaId;
  title: string;
  description: string;
  startSlug: string;
  troubleshootingSlug: string;
  items: PersonaNavItem[];
};

export type DocsNavigation = {
  personas: PersonaNavSection[];
};

export type SearchIndexItem = {
  id: string;
  title: string;
  summary: string;
  persona: PersonaId;
  path: string;
  headings: string[];
  keywords: string[];
};

export type Breadcrumb = {
  label: string;
  href: string;
};
