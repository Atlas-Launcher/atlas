import { cache } from "react";
import path from "node:path";
import { promises as fs } from "node:fs";

import { extractHeadings } from "@/lib/docs/markdown";
import {
  DOC_INTENTS,
  PERSONAS,
  type Breadcrumb,
  type DocFrontmatter,
  type DocItem,
  type DocsNavigation,
  type PersonaId,
  type PersonaNavSection,
  type SearchIndexItem,
} from "@/lib/docs/types";

const USER_DOCS_DIR = path.join("docs", "user");
const NAV_FILE = "navigation.json";

type DocsData = {
  navigation: DocsNavigation;
  personaMap: Record<PersonaId, PersonaNavSection>;
  docsByRoute: Map<string, DocItem>;
  docsByRelativePath: Map<string, DocItem>;
  orderedDocsByPersona: Record<PersonaId, DocItem[]>;
};

function parseString(value: string) {
  const normalized = value.trim();
  if ((normalized.startsWith('"') && normalized.endsWith('"')) || (normalized.startsWith("'") && normalized.endsWith("'"))) {
    return normalized.slice(1, -1);
  }
  return normalized;
}

function parseFrontmatter(raw: string, filePath: string): { frontmatter: DocFrontmatter; body: string } {
  if (!raw.startsWith("---\n")) {
    throw new Error(`Missing frontmatter in ${filePath}`);
  }

  const endIndex = raw.indexOf("\n---\n", 4);
  if (endIndex === -1) {
    throw new Error(`Unterminated frontmatter block in ${filePath}`);
  }

  const block = raw.slice(4, endIndex);
  const body = raw.slice(endIndex + 5).trimStart();
  const fields: Record<string, string> = {};

  for (const line of block.split("\n")) {
    const trimmed = line.trim();
    if (trimmed.length === 0 || trimmed.startsWith("#")) {
      continue;
    }

    const separatorIndex = trimmed.indexOf(":");
    if (separatorIndex === -1) {
      throw new Error(`Invalid frontmatter line '${line}' in ${filePath}`);
    }

    const key = trimmed.slice(0, separatorIndex).trim();
    const value = trimmed.slice(separatorIndex + 1).trim();
    fields[key] = value;
  }

  const required = ["title", "summary", "persona", "order", "keywords", "intent"];
  for (const key of required) {
    if (!fields[key]) {
      throw new Error(`Missing frontmatter field '${key}' in ${filePath}`);
    }
  }

  const persona = parseString(fields.persona) as PersonaId;
  if (!PERSONAS.includes(persona)) {
    throw new Error(`Invalid persona '${fields.persona}' in ${filePath}`);
  }

  const order = Number(fields.order);
  if (!Number.isFinite(order)) {
    throw new Error(`Frontmatter field 'order' must be numeric in ${filePath}`);
  }

  let keywords: string[];
  try {
    const parsedKeywords = JSON.parse(fields.keywords);
    if (!Array.isArray(parsedKeywords) || parsedKeywords.some((item) => typeof item !== "string")) {
      throw new Error("keywords must be an array of strings");
    }
    keywords = parsedKeywords;
  } catch (error) {
    throw new Error(
      `Invalid keywords array in ${filePath}. Use JSON array syntax like ["a", "b"]. ${(error as Error).message}`
    );
  }

  const intent = parseString(fields.intent);
  if (!DOC_INTENTS.includes(intent as (typeof DOC_INTENTS)[number])) {
    throw new Error(`Invalid intent '${fields.intent}' in ${filePath}`);
  }

  return {
    frontmatter: {
      title: parseString(fields.title),
      summary: parseString(fields.summary),
      persona,
      order,
      keywords,
      intent: intent as DocFrontmatter["intent"],
    },
    body,
  };
}

function normalizeRelativePath(value: string) {
  return value.replaceAll("\\", "/");
}

function filePathToRoute(relativePath: string) {
  const normalized = normalizeRelativePath(relativePath);
  const [persona, ...rest] = normalized.split("/");
  const fileName = rest.pop();

  if (!fileName || !PERSONAS.includes(persona as PersonaId)) {
    return null;
  }

  const baseName = fileName.replace(/\.md$/i, "");
  const slugParts = [...rest];
  if (baseName.toLowerCase() !== "readme") {
    slugParts.push(baseName);
  }

  const slug = slugParts.join("/");
  return {
    persona: persona as PersonaId,
    slug,
    routePath: slug.length > 0 ? `/docs/${persona}/${slug}` : `/docs/${persona}`,
  };
}

async function readNavigation(docsRoot: string) {
  const navigationPath = path.join(docsRoot, NAV_FILE);
  const raw = await fs.readFile(navigationPath, "utf-8");

  let navigation: DocsNavigation;
  try {
    navigation = JSON.parse(raw) as DocsNavigation;
  } catch (error) {
    throw new Error(`Invalid JSON in ${navigationPath}: ${(error as Error).message}`);
  }

  if (!Array.isArray(navigation.personas) || navigation.personas.length === 0) {
    throw new Error(`navigation.json must define a non-empty personas array (${navigationPath})`);
  }

  return navigation;
}

async function resolveDocsRoot() {
  const candidates = [
    path.resolve(process.cwd(), USER_DOCS_DIR),
    path.resolve(process.cwd(), "..", "..", USER_DOCS_DIR),
  ];

  for (const candidate of candidates) {
    try {
      const stat = await fs.stat(candidate);
      if (stat.isDirectory()) {
        return candidate;
      }
    } catch {
      continue;
    }
  }

  throw new Error(`Unable to resolve docs root (${USER_DOCS_DIR}) from cwd ${process.cwd()}`);
}

const loadDocsData = cache(async (): Promise<DocsData> => {
  const docsRoot = await resolveDocsRoot();
  const navigation = await readNavigation(docsRoot);

  const personaMap: Record<PersonaId, PersonaNavSection> = {
    player: null as never,
    creator: null as never,
    host: null as never,
  };

  for (const section of navigation.personas) {
    if (!PERSONAS.includes(section.id)) {
      throw new Error(`navigation.json includes unknown persona '${section.id}'`);
    }
    personaMap[section.id] = section;
  }

  for (const persona of PERSONAS) {
    if (!personaMap[persona]) {
      throw new Error(`navigation.json is missing persona section '${persona}'`);
    }
  }

  const docsByRelativePath = new Map<string, DocItem>();
  const docsByRoute = new Map<string, DocItem>();
  const orderedDocsByPersona: Record<PersonaId, DocItem[]> = {
    player: [],
    creator: [],
    host: [],
  };

  for (const section of navigation.personas) {
    for (const navItem of section.items) {
      const relativePath = normalizeRelativePath(navItem.file);
      const absolutePath = path.join(docsRoot, relativePath);

      let raw: string;
      try {
        raw = await fs.readFile(absolutePath, "utf-8");
      } catch {
        throw new Error(`navigation.json references missing file: ${relativePath}`);
      }

      const { frontmatter, body } = parseFrontmatter(raw, absolutePath);

      if (frontmatter.persona !== section.id) {
        throw new Error(
          `${relativePath} persona mismatch: frontmatter is '${frontmatter.persona}' but nav section is '${section.id}'`
        );
      }

      const route = filePathToRoute(relativePath);
      if (!route) {
        throw new Error(`Unable to compute route for ${relativePath}`);
      }

      const headings = extractHeadings(body);
      const doc: DocItem = {
        id: `${section.id}:${route.slug.length > 0 ? route.slug : "index"}`,
        title: frontmatter.title,
        summary: frontmatter.summary,
        persona: frontmatter.persona,
        order: frontmatter.order,
        keywords: frontmatter.keywords,
        intent: frontmatter.intent,
        relativePath,
        slug: route.slug,
        routePath: route.routePath,
        body,
        headings,
      };

      if (navItem.slug !== route.slug) {
        throw new Error(
          `navigation slug mismatch for ${relativePath}: expected '${route.slug}', got '${navItem.slug}'`
        );
      }

      docsByRelativePath.set(relativePath, doc);
      docsByRoute.set(doc.routePath, doc);
      orderedDocsByPersona[section.id].push(doc);
    }

    orderedDocsByPersona[section.id].sort((a, b) => a.order - b.order || a.title.localeCompare(b.title));
  }

  return {
    navigation,
    personaMap,
    docsByRoute,
    docsByRelativePath,
    orderedDocsByPersona,
  };
});

export async function assertDocsConfiguration() {
  await loadDocsData();
}

export async function getDocsNavigation() {
  const data = await loadDocsData();
  return data.navigation;
}

export async function getPersonaSection(persona: PersonaId) {
  const data = await loadDocsData();
  return data.personaMap[persona];
}

export async function getPersonaDocs(persona: PersonaId) {
  const data = await loadDocsData();
  return data.orderedDocsByPersona[persona];
}

export async function getDocByRoute(persona: PersonaId, slugParts: string[]) {
  const slug = slugParts.join("/");
  const routePath = slug.length > 0 ? `/docs/${persona}/${slug}` : `/docs/${persona}`;
  const data = await loadDocsData();
  return data.docsByRoute.get(routePath) ?? null;
}

export async function getAllDocParams() {
  const data = await loadDocsData();
  const params: Array<{ persona: PersonaId; slug: string[] }> = [];

  for (const persona of PERSONAS) {
    for (const doc of data.orderedDocsByPersona[persona]) {
      if (doc.slug.length === 0) {
        continue;
      }
      params.push({ persona, slug: doc.slug.split("/") });
    }
  }

  return params;
}

export async function getAdjacentDocs(persona: PersonaId, slugParts: string[]) {
  const slug = slugParts.join("/");
  const data = await loadDocsData();
  const docs = data.orderedDocsByPersona[persona];
  const currentIndex = docs.findIndex((doc) => doc.slug === slug);

  return {
    previous: currentIndex > 0 ? docs[currentIndex - 1] : null,
    next: currentIndex >= 0 && currentIndex < docs.length - 1 ? docs[currentIndex + 1] : null,
  };
}

export async function getSearchIndex(): Promise<SearchIndexItem[]> {
  const data = await loadDocsData();
  const items: SearchIndexItem[] = [];

  for (const persona of PERSONAS) {
    for (const doc of data.orderedDocsByPersona[persona]) {
      items.push({
        id: doc.id,
        title: doc.title,
        summary: doc.summary,
        persona: doc.persona,
        path: doc.routePath,
        headings: doc.headings.map((heading) => heading.text),
        keywords: doc.keywords,
      });
    }
  }

  return items;
}

export async function getBreadcrumbs(persona: PersonaId, slugParts: string[]): Promise<Breadcrumb[]> {
  const section = await getPersonaSection(persona);
  const doc = await getDocByRoute(persona, slugParts);

  const breadcrumbs: Breadcrumb[] = [
    { label: "Docs", href: "/docs" },
    { label: section.title, href: `/docs/${persona}` },
  ];

  if (doc) {
    breadcrumbs.push({ label: doc.title, href: doc.routePath });
  }

  return breadcrumbs;
}
