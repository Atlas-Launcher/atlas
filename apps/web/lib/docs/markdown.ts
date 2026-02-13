import type { DocHeading } from "@/lib/docs/types";

function escapeHtml(input: string) {
  return input
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

export function slugifyHeading(text: string) {
  return text
    .toLowerCase()
    .trim()
    .replace(/[`*_~]/g, "")
    .replace(/[^a-z0-9\s-]/g, "")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-");
}

function renderInline(input: string) {
  let html = escapeHtml(input);
  html = html.replace(/`([^`]+)`/g, "<code>$1</code>");
  html = html.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  html = html.replace(/\*([^*]+)\*/g, "<em>$1</em>");
  html = html.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>');
  return html;
}

export function extractHeadings(markdown: string): DocHeading[] {
  const headings: DocHeading[] = [];
  for (const rawLine of markdown.split(/\r?\n/)) {
    const line = rawLine.trim();
    const match = line.match(/^(#{1,3})\s+(.+)$/);
    if (!match) {
      continue;
    }

    const level = match[1].length;
    if (level === 1) {
      continue;
    }

    const text = match[2].trim();
    headings.push({
      level,
      text,
      id: slugifyHeading(text),
    });
  }
  return headings;
}

function flushParagraph(buffer: string[], output: string[]) {
  if (buffer.length === 0) {
    return;
  }
  const text = buffer.join(" ").trim();
  if (text.length > 0) {
    output.push(`<p>${renderInline(text)}</p>`);
  }
  buffer.length = 0;
}

export function renderMarkdown(markdown: string) {
  const lines = markdown.split(/\r?\n/);
  const output: string[] = [];
  const paragraphBuffer: string[] = [];
  let inCodeBlock = false;
  let codeFenceLang = "";
  let inUnorderedList = false;
  let inOrderedList = false;

  const closeLists = () => {
    if (inUnorderedList) {
      output.push("</ul>");
      inUnorderedList = false;
    }
    if (inOrderedList) {
      output.push("</ol>");
      inOrderedList = false;
    }
  };

  for (const rawLine of lines) {
    const line = rawLine.replace(/\t/g, "  ");
    const trimmed = line.trim();

    if (trimmed.startsWith("```")) {
      flushParagraph(paragraphBuffer, output);
      closeLists();
      if (inCodeBlock) {
        output.push("</code></pre>");
        inCodeBlock = false;
        codeFenceLang = "";
      } else {
        codeFenceLang = trimmed.slice(3).trim();
        const className = codeFenceLang.length > 0 ? ` class="language-${escapeHtml(codeFenceLang)}"` : "";
        output.push(`<pre><code${className}>`);
        inCodeBlock = true;
      }
      continue;
    }

    if (inCodeBlock) {
      output.push(`${escapeHtml(rawLine)}\n`);
      continue;
    }

    if (trimmed.length === 0) {
      flushParagraph(paragraphBuffer, output);
      closeLists();
      continue;
    }

    const headingMatch = trimmed.match(/^(#{1,3})\s+(.+)$/);
    if (headingMatch) {
      flushParagraph(paragraphBuffer, output);
      closeLists();
      const level = headingMatch[1].length;
      if (level === 1) {
        continue;
      }
      const text = headingMatch[2].trim();
      output.push(`<h${level} id="${slugifyHeading(text)}">${renderInline(text)}</h${level}>`);
      continue;
    }

    const unorderedMatch = trimmed.match(/^-\s+(.+)$/);
    if (unorderedMatch) {
      flushParagraph(paragraphBuffer, output);
      if (!inUnorderedList) {
        closeLists();
        output.push("<ul>");
        inUnorderedList = true;
      }
      output.push(`<li>${renderInline(unorderedMatch[1])}</li>`);
      continue;
    }

    const orderedMatch = trimmed.match(/^\d+\.\s+(.+)$/);
    if (orderedMatch) {
      flushParagraph(paragraphBuffer, output);
      if (!inOrderedList) {
        closeLists();
        output.push("<ol>");
        inOrderedList = true;
      }
      output.push(`<li>${renderInline(orderedMatch[1])}</li>`);
      continue;
    }

    paragraphBuffer.push(trimmed);
  }

  flushParagraph(paragraphBuffer, output);
  closeLists();

  if (inCodeBlock) {
    output.push("</code></pre>");
  }

  return output.join("\n");
}
