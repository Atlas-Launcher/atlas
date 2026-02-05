import fs from "node:fs/promises";
import path from "node:path";

export type TemplateFile = {
  path: string;
  content: string;
};

const TEMPLATE_ROOT = path.join(process.cwd(), "templates", "atlas-pack");

async function collectFiles(dir: string, prefix = ""): Promise<TemplateFile[]> {
  const entries = await fs.readdir(dir, { withFileTypes: true });
  const files: TemplateFile[] = [];

  for (const entry of entries) {
    if (entry.name === ".DS_Store") {
      continue;
    }
    const fullPath = path.join(dir, entry.name);
    const relativePath = prefix ? `${prefix}/${entry.name}` : entry.name;

    if (entry.isDirectory()) {
      files.push(...(await collectFiles(fullPath, relativePath)));
      continue;
    }

    const content = await fs.readFile(fullPath, "utf8");
    files.push({ path: relativePath, content });
  }

  return files;
}

export async function getAtlasPackTemplateFiles() {
  return collectFiles(TEMPLATE_ROOT);
}
