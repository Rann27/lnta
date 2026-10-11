import { describe, it, expect, vi } from "vitest";
import { PreviewTransport } from "./transport";
import { resolveScenarioName, createPreviewScenarios } from "./scenarios";
import type { EngineEvent } from "../bindings";

describe("Preview scenarios and transport", () => {
  it("resolves all 6 scenario names and their aliases", () => {
    expect(resolveScenarioName("no-projects")).toBe("no-projects");
    expect(resolveScenarioName("empty")).toBe("no-projects");
    expect(resolveScenarioName("project-idle-blocker")).toBe("project-idle-blocker");
    expect(resolveScenarioName("preflight-blockers")).toBe("project-idle-blocker");
    expect(resolveScenarioName("run-in-progress")).toBe("run-in-progress");
    expect(resolveScenarioName("streaming-tokens")).toBe("run-in-progress");
    expect(resolveScenarioName("chapter-needs-attention")).toBe("chapter-needs-attention");
    expect(resolveScenarioName("needs-attention")).toBe("chapter-needs-attention");
    expect(resolveScenarioName("run-paused-blocker")).toBe("run-paused-blocker");
    expect(resolveScenarioName("paused")).toBe("run-paused-blocker");
    expect(resolveScenarioName("qa-finished")).toBe("qa-finished");
    expect(resolveScenarioName("qa-done")).toBe("qa-finished");
    expect(resolveScenarioName(null)).toBe("run-in-progress");
    expect(resolveScenarioName("unknown-name")).toBe("run-in-progress");
  });

  it("creates valid data structures for all 6 scenarios", () => {
    const scenarios = createPreviewScenarios();
    const names = [
      "no-projects",
      "project-idle-blocker",
      "run-in-progress",
      "chapter-needs-attention",
      "run-paused-blocker",
      "qa-finished",
    ];

    for (const name of names) {
      const sc = scenarios[name];
      expect(sc).toBeDefined();
      expect(sc.name).toBe(name);
      expect(sc.projectConfig).toBeDefined();
      expect(sc.preflight).toBeDefined();
      expect(sc.runState).toBeDefined();
    }
  });

  it("handles scenario no-projects correctly", async () => {
    const transport = new PreviewTransport("no-projects");
    const recents = await transport.listRecentProjects();
    expect(recents).toEqual([]);

    const runState = await transport.getRunState("");
    expect(runState.active).toBe(false);
    expect(runState.state).toBe("idle");
  });

  it("handles scenario project-idle-blocker correctly", async () => {
    const transport = new PreviewTransport("project-idle-blocker");
    const recents = await transport.listRecentProjects();
    expect(recents.length).toBe(1);

    const preflight = await transport.runPreflight(recents[0].path);
    expect(preflight.can_run).toBe(false);
    expect(preflight.items.some((item) => item.status === "blocker")).toBe(true);

    const runState = await transport.getRunState(recents[0].path);
    expect(runState.state).toBe("idle");
    expect(runState.active).toBe(false);
  });

  it("handles scenario run-in-progress with streaming tokens", async () => {
    vi.useFakeTimers();
    const transport = new PreviewTransport("run-in-progress");
    const runState = await transport.getRunState("");
    expect(runState.state).toBe("running");
    expect(runState.active).toBe(true);
    expect(runState.current_chapter_id).toBe("chap_03");

    const events: EngineEvent[] = [];
    const unlisten = await transport.onEngineEvent((ev) => {
      events.push(ev);
    });

    vi.advanceTimersByTime(1000);
    expect(events.length).toBeGreaterThan(0);
    expect(events.some((e) => e.event === "call:progress")).toBe(true);

    unlisten();
    vi.useRealTimers();
  });

  it("handles scenario chapter-needs-attention correctly", async () => {
    const transport = new PreviewTransport("chapter-needs-attention");
    const chapters = await transport.listChapters("");
    const attentionChapter = chapters.find((c) => c.id === "chap_03");
    expect(attentionChapter).toBeDefined();
    expect(attentionChapter?.status).toBe("needs_attention");

    const runState = await transport.getRunState("");
    expect(runState.state).toBe("needs_attention");
  });

  it("handles scenario run-paused-blocker correctly", async () => {
    const transport = new PreviewTransport("run-paused-blocker");
    const runState = await transport.getRunState("");
    expect(runState.state).toBe("paused");
    expect(runState.active).toBe(false);

    const preflight = await transport.runPreflight("");
    expect(preflight.can_run).toBe(false);
    // Preview reports use the ids and codes the Rust preflight sends.
    expect(preflight.items.some((i) => i.message_code === "preflight.template_unreadable")).toBe(
      true
    );
  });

  it("handles scenario qa-finished correctly", async () => {
    const transport = new PreviewTransport("qa-finished");
    const chapters = await transport.listChapters("");
    expect(chapters.every((c) => c.status === "done")).toBe(true);

    const runState = await transport.getRunState("");
    expect(runState.state).toBe("done");
  });

  it("supports start, pause, resume, and stop run state transitions", async () => {
    const transport = new PreviewTransport("project-idle-blocker");
    const events: EngineEvent[] = [];
    await transport.onEngineEvent((ev) => {
      events.push(ev);
    });

    const runId = await transport.startRun("D:/test");
    expect(runId).toContain("run_preview_");
    expect((await transport.getRunState("")).state).toBe("running");

    await transport.pauseRun("D:/test");
    expect((await transport.getRunState("")).state).toBe("paused");

    await transport.resumeRun("D:/test");
    expect((await transport.getRunState("")).state).toBe("running");

    await transport.stopRun("D:/test");
    expect((await transport.getRunState("")).state).toBe("idle");

    expect(events.some((e) => e.event === "run:state")).toBe(true);
  });

  it("supports chapter, segment, translation, and glossary CRUD queries", async () => {
    const transport = new PreviewTransport("run-in-progress");
    const chapters = await transport.listChapters("");
    expect(chapters.length).toBeGreaterThan(0);

    const detail = await transport.getChapter("", chapters[0].id);
    expect(detail.chapter.id).toBe(chapters[0].id);
    expect(detail.segments.length).toBeGreaterThan(0);

    const segments = await transport.listSegments("", chapters[0].id);
    expect(segments.length).toBe(detail.segments.length);

    const glossary = await transport.listGlossary("");
    expect(glossary.length).toBeGreaterThan(0);

    const added = await transport.addGlossaryEntry("", {
      source: "勇者",
      translation: "Pahlawan",
      note: "Gelar",
    });
    expect(added.source).toBe("勇者");
    expect((await transport.listGlossary("")).some((g) => g.source === "勇者")).toBe(true);

    await transport.updateGlossaryEntry("", {
      source: "勇者",
      translation: "Sang Pahlawan",
      note: "Gelar kehormatan",
    });
    const updated = (await transport.listGlossary("")).find((g) => g.source === "勇者");
    expect(updated?.translation).toBe("Sang Pahlawan");

    await transport.deleteGlossaryEntry("", "勇者");
    expect((await transport.listGlossary("")).some((g) => g.source === "勇者")).toBe(false);
  });
});
