"use client";

import { useId, useState, type ReactNode } from "react";
import { PanelLeftClose, PanelLeftOpen } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

type DocsShellProps = {
  sidebar: ReactNode;
  children: ReactNode;
};

export default function DocsShell({ sidebar, children }: DocsShellProps) {
  const sidebarId = useId();
  const [isSidebarOpen, setIsSidebarOpen] = useState(true);

  const onToggleSidebar = () => {
    setIsSidebarOpen((current) => !current);
  };

  return (
    <div
      className={cn(
        "space-y-6 pb-8 pt-6 lg:grid lg:space-y-0 lg:gap-6",
        isSidebarOpen
          ? "lg:grid-cols-[300px_minmax(0,1fr)] lg:items-start"
          : "lg:grid-cols-[minmax(0,1fr)]"
      )}
    >
      <aside
        id={sidebarId}
        className={cn(
          "space-y-6",
          isSidebarOpen ? "lg:sticky lg:top-24" : "lg:hidden"
        )}
      >
        {sidebar}
      </aside>

      <section className="min-w-0">
        <div className="mb-3 hidden justify-end lg:flex">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={onToggleSidebar}
            aria-controls={sidebarId}
            aria-expanded={isSidebarOpen}
          >
            {isSidebarOpen ? (
              <PanelLeftClose className="h-4 w-4" aria-hidden="true" />
            ) : (
              <PanelLeftOpen className="h-4 w-4" aria-hidden="true" />
            )}
            {isSidebarOpen ? "Hide sidebar" : "Show sidebar"}
          </Button>
        </div>

        {children}
      </section>
    </div>
  );
}
