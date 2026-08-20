// Platform admin: level-document templates + the stamp-position editor.
// Desktop, mouse-driven (draw to place, drag to move, corner handles to
// resize; every gesture persists immediately). Coordinates are stored in PDF
// points, origin bottom-left, (x, y) = bottom-left corner of the box.
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { Navigate } from "react-router";
import * as pdfjsLib from "pdfjs-dist";
import pdfWorkerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";

import { pushErrorToast, pushToast } from "../../components/toasts";
import {
  Badge,
  Button,
  Card,
  ErrorState,
  Loading,
  PageTitle,
} from "../../components/ui";
import { t } from "../../i18n/fr";
import { api, type StampPosition, type TreeSkill } from "../../lib/api";
import { translateApiError } from "../../lib/errors";
import { useMe } from "../../lib/session";

pdfjsLib.GlobalWorkerOptions.workerSrc = pdfWorkerUrl;

interface CanvasBox {
  skillId: string;
  x: number;
  y: number;
  width: number;
  height: number;
  fontSize: number;
  page: number;
}

type Gesture =
  | { kind: "idle" }
  | { kind: "placing"; skillId: string }
  | { kind: "drawing"; skillId: string; startX: number; startY: number; x: number; y: number; width: number; height: number }
  | { kind: "moving"; skillId: string; offsetX: number; offsetY: number }
  | { kind: "resizing"; skillId: string; corner: "nw" | "ne" | "sw" | "se" };

const HANDLE_RADIUS = 8;

export default function LevelDocuments() {
  const { data: me } = useMe();
  const queryClient = useQueryClient();
  const [level, setLevel] = useState<string | null>(null);
  const [pageNumber, setPageNumber] = useState(1);
  const [scale, setScale] = useState(1);
  const [pdfDoc, setPdfDoc] = useState<pdfjsLib.PDFDocumentProxy | null>(null);
  const [pdfSize, setPdfSize] = useState<{ width: number; height: number } | null>(null);
  const [gesture, setGesture] = useState<Gesture>({ kind: "idle" });
  const [draft, setDraft] = useState<CanvasBox | null>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const overlayRef = useRef<HTMLDivElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const levels = useQuery({
    queryKey: ["competencies", "levels"],
    queryFn: () => api.competencies.levels(),
  });
  const activeLevel =
    level ?? levels.data?.find((l) => l.has_document)?.certification ??
    levels.data?.[0]?.certification ?? null;
  const summary = levels.data?.find((l) => l.certification === activeLevel);

  const tree = useQuery({
    queryKey: ["competencies", "tree", activeLevel],
    queryFn: () => api.competencies.tree(activeLevel!),
    enabled: activeLevel != null,
  });
  const positions = useQuery({
    queryKey: ["competencies", "positions", activeLevel],
    queryFn: () => api.competencies.positions(activeLevel!),
    enabled: activeLevel != null && summary?.has_document === true,
    retry: false,
  });

  // Load the PDF whenever the level's document changes (uploads bump the
  // reload key: page_count alone can miss a same-size replacement).
  const [reloadKey, setReloadKey] = useState(0);
  const documentVersion = `${activeLevel}:${summary?.has_document ?? false}:${summary?.page_count ?? 0}:${reloadKey}`;
  useEffect(() => {
    let cancelled = false;
    setPdfDoc(null);
    setPdfSize(null);
    setPageNumber(1);
    if (!activeLevel || !summary?.has_document) return;
    (async () => {
      try {
        const blob = await api.competencies.downloadDocument(activeLevel);
        if (!blob || cancelled) return;
        const data = await blob.arrayBuffer();
        const doc = await pdfjsLib.getDocument({ data: data.slice(0) }).promise;
        if (!cancelled) setPdfDoc(doc);
      } catch (e) {
        if (!cancelled) pushErrorToast(translateApiError(e));
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [documentVersion]);

  // Render the current page at the current zoom.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      if (!pdfDoc || !canvasRef.current) return;
      const page = await pdfDoc.getPage(pageNumber);
      if (cancelled) return;
      const base = page.getViewport({ scale: 1 });
      setPdfSize({ width: base.width, height: base.height });
      const viewport = page.getViewport({ scale });
      const canvas = canvasRef.current;
      canvas.width = viewport.width;
      canvas.height = viewport.height;
      await page.render({ canvas, viewport }).promise;
    })();
    return () => {
      cancelled = true;
    };
  }, [pdfDoc, pageNumber, scale]);

  const save = useMutation({
    meta: { inlineError: true },
    mutationFn: ({ skillId, box }: { skillId: string; box: CanvasBox }) => {
      // Canvas (top-left origin) → PDF points (bottom-left origin/anchor).
      const height = pdfSize?.height ?? 0;
      return api.competencies.savePosition(activeLevel!, skillId, {
        page: box.page,
        x: Math.round(box.x / scale),
        y: Math.round(height - (box.y + box.height) / scale),
        width: Math.round(box.width / scale),
        height: Math.round(box.height / scale),
        font_size: box.fontSize,
      });
    },
    onSuccess: () => {
      pushToast(t.competencies.positionSaved);
      queryClient.invalidateQueries({
        queryKey: ["competencies", "positions", activeLevel],
      });
    },
    onError: (e) => pushErrorToast(translateApiError(e)),
  });
  const removePosition = useMutation({
    meta: { inlineError: true },
    mutationFn: (skillId: string) =>
      api.competencies.deletePosition(activeLevel!, skillId),
    onSuccess: () => {
      pushToast(t.competencies.positionDeleted);
      queryClient.invalidateQueries({
        queryKey: ["competencies", "positions", activeLevel],
      });
    },
    onError: (e) => pushErrorToast(translateApiError(e)),
  });
  const upload = useMutation({
    meta: { inlineError: true },
    mutationFn: async (file: File) => {
      const buffer = await file.arrayBuffer();
      let binary = "";
      const bytes = new Uint8Array(buffer);
      const chunk = 0x8000;
      for (let i = 0; i < bytes.length; i += chunk) {
        binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
      }
      return api.competencies.uploadDocument(activeLevel!, {
        file_name: file.name,
        pdf_base64: btoa(binary),
      });
    },
    onSuccess: () => {
      pushToast(t.competencies.documentUploaded);
      setReloadKey((k) => k + 1);
      queryClient.invalidateQueries({ queryKey: ["competencies", "levels"] });
      queryClient.invalidateQueries({
        queryKey: ["competencies", "positions", activeLevel],
      });
    },
    onError: (e) => pushErrorToast(translateApiError(e)),
  });
  const removeDocument = useMutation({
    meta: { inlineError: true },
    mutationFn: () => api.competencies.deleteDocument(activeLevel!),
    onSuccess: () => {
      setPdfDoc(null);
      queryClient.invalidateQueries({ queryKey: ["competencies", "levels"] });
    },
    onError: (e) => pushErrorToast(translateApiError(e)),
  });

  // Boxes of the current page in canvas coordinates.
  const pdfHeight = pdfSize?.height ?? 0;
  const toCanvas = useCallback(
    (p: StampPosition): CanvasBox => ({
      skillId: p.skill_id,
      x: p.x * scale,
      y: (pdfHeight - p.y - p.height) * scale,
      width: p.width * scale,
      height: p.height * scale,
      fontSize: p.font_size,
      page: p.page,
    }),
    [scale, pdfHeight],
  );
  const skills: TreeSkill[] =
    tree.data?.domains.flatMap((d) => d.modules.flatMap((m) => m.skills)) ?? [];
  const skillName = (id: string) =>
    skills.find((s) => s.id === id)?.name ?? "?";
  const allPositions = positions.data ?? [];
  const pageBoxes: CanvasBox[] = allPositions
    .filter((p) => p.page === pageNumber && !p.out_of_range)
    .map(toCanvas)
    .map((box) => (draft && draft.skillId === box.skillId ? draft : box));
  if (
    draft &&
    gesture.kind === "drawing" &&
    !pageBoxes.some((b) => b.skillId === draft.skillId)
  ) {
    pageBoxes.push(draft);
  }
  const placedIds = new Set(allPositions.map((p) => p.skill_id));
  const unplaced = skills.filter((s) => !placedIds.has(s.id));

  const mousePoint = (e: React.MouseEvent) => {
    const rect = overlayRef.current!.getBoundingClientRect();
    return { x: e.clientX - rect.left, y: e.clientY - rect.top };
  };
  const cornerAt = (box: CanvasBox, x: number, y: number) => {
    const corners = {
      nw: [box.x, box.y],
      ne: [box.x + box.width, box.y],
      sw: [box.x, box.y + box.height],
      se: [box.x + box.width, box.y + box.height],
    } as const;
    for (const [corner, [cx, cy]] of Object.entries(corners)) {
      if (Math.abs(x - cx) <= HANDLE_RADIUS && Math.abs(y - cy) <= HANDLE_RADIUS) {
        return corner as "nw" | "ne" | "sw" | "se";
      }
    }
    return null;
  };

  const onMouseDown = (e: React.MouseEvent) => {
    const { x, y } = mousePoint(e);
    if (gesture.kind === "placing") {
      setGesture({
        kind: "drawing",
        skillId: gesture.skillId,
        startX: x,
        startY: y,
        x,
        y,
        width: 0,
        height: 0,
      });
      setDraft({
        skillId: gesture.skillId,
        x,
        y,
        width: 0,
        height: 0,
        fontSize: 8,
        page: pageNumber,
      });
      return;
    }
    for (const box of pageBoxes) {
      const corner = cornerAt(box, x, y);
      if (corner) {
        setDraft(box);
        setGesture({ kind: "resizing", skillId: box.skillId, corner });
        return;
      }
      if (
        x >= box.x &&
        x <= box.x + box.width &&
        y >= box.y &&
        y <= box.y + box.height
      ) {
        setDraft(box);
        setGesture({
          kind: "moving",
          skillId: box.skillId,
          offsetX: x - box.x,
          offsetY: y - box.y,
        });
        return;
      }
    }
  };

  const onMouseMove = (e: React.MouseEvent) => {
    if (gesture.kind === "idle" || gesture.kind === "placing" || !draft) return;
    const { x, y } = mousePoint(e);
    if (gesture.kind === "drawing") {
      setDraft({
        ...draft,
        x: Math.min(gesture.startX, x),
        y: Math.min(gesture.startY, y),
        width: Math.abs(x - gesture.startX),
        height: Math.abs(y - gesture.startY),
      });
    } else if (gesture.kind === "moving") {
      setDraft({ ...draft, x: x - gesture.offsetX, y: y - gesture.offsetY });
    } else if (gesture.kind === "resizing") {
      const right = draft.x + draft.width;
      const bottom = draft.y + draft.height;
      let { x: bx, y: by } = draft;
      let width = draft.width;
      let height = draft.height;
      if (gesture.corner.includes("w")) {
        bx = Math.min(x, right - 20);
        width = right - bx;
      } else {
        width = Math.max(20, x - draft.x);
      }
      if (gesture.corner.includes("n")) {
        by = Math.min(y, bottom - 10);
        height = bottom - by;
      } else {
        height = Math.max(10, y - draft.y);
      }
      setDraft({ ...draft, x: bx, y: by, width, height });
    }
  };

  const finishGesture = () => {
    if (!draft) {
      setGesture({ kind: "idle" });
      return;
    }
    if (gesture.kind === "drawing") {
      // Tiny accidental drags are discarded (Fosse convention).
      if (draft.width > 10 && draft.height > 5) {
        save.mutate({ skillId: draft.skillId, box: draft });
      }
    } else if (gesture.kind === "moving" || gesture.kind === "resizing") {
      save.mutate({ skillId: draft.skillId, box: draft });
    }
    setDraft(null);
    setGesture({ kind: "idle" });
  };

  if (me && (me.impersonating || !me.user.is_global_admin)) {
    return <Navigate to="/" replace />;
  }
  if (levels.isPending) return <Loading />;
  if (levels.isError) return <ErrorState onRetry={() => levels.refetch()} />;

  return (
    <>
      <PageTitle>{t.competencies.documentsTitle}</PageTitle>
      <p className="mb-4 text-sm text-ink-soft">{t.competencies.documentsHint}</p>

      {/* Level tiles */}
      <div className="mb-4 flex flex-wrap gap-2">
        {(levels.data ?? []).map((l) => (
          <button
            key={l.certification}
            onClick={() => {
              setLevel(l.certification);
              setGesture({ kind: "idle" });
            }}
            className={`min-h-11 rounded-(--radius-control) border px-3 text-sm font-semibold ${
              l.certification === activeLevel
                ? "border-abysse-600 bg-abysse-600 text-white"
                : l.has_document
                  ? "border-success-600/40 bg-success-50 text-success-600"
                  : "border-hairline bg-white text-ink-soft"
            }`}
          >
            {l.certification}
            {l.has_document && ` • ${t.competencies.pages(l.page_count ?? 0)}`}
          </button>
        ))}
      </div>

      <Card className="mb-4">
        <div className="flex flex-wrap items-center gap-2">
          <input
            ref={fileInputRef}
            type="file"
            accept=".pdf,application/pdf"
            className="hidden"
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file) upload.mutate(file);
              e.target.value = "";
            }}
          />
          <Button
            variant="secondary"
            disabled={upload.isPending || !activeLevel}
            onClick={() => fileInputRef.current?.click()}
          >
            📤 {summary?.has_document ? t.competencies.replace : t.competencies.upload}
          </Button>
          {summary?.has_document && (
            <Button
              variant="danger"
              disabled={removeDocument.isPending}
              onClick={() => {
                if (
                  window.confirm(
                    t.competencies.deleteDocumentConfirm(activeLevel ?? ""),
                  )
                ) {
                  removeDocument.mutate();
                }
              }}
            >
              {t.competencies.deleteDocument}
            </Button>
          )}
          {!summary?.has_document && (
            <Badge tone="neutral">{t.competencies.noDocument}</Badge>
          )}
        </div>
      </Card>

      {summary?.has_document && pdfDoc && (
        <div className="grid gap-4 lg:grid-cols-3">
          <div className="lg:col-span-2">
            <div className="mb-2 flex flex-wrap items-center gap-2 text-sm">
              <Button
                variant="secondary"
                disabled={pageNumber <= 1}
                onClick={() => setPageNumber((p) => p - 1)}
              >
                ←
              </Button>
              <span className="font-semibold">
                {t.competencies.page(pageNumber, pdfDoc.numPages)}
              </span>
              <Button
                variant="secondary"
                disabled={pageNumber >= pdfDoc.numPages}
                onClick={() => setPageNumber((p) => p + 1)}
              >
                →
              </Button>
              <span className="ml-auto flex items-center gap-1">
                <Button
                  variant="secondary"
                  disabled={scale <= 0.5}
                  onClick={() => setScale((s) => Math.max(0.5, s - 0.25))}
                >
                  −
                </Button>
                <span className="w-14 text-center font-semibold">
                  {Math.round(scale * 100)}%
                </span>
                <Button
                  variant="secondary"
                  disabled={scale >= 2}
                  onClick={() => setScale((s) => Math.min(2, s + 0.25))}
                >
                  +
                </Button>
              </span>
            </div>
            {gesture.kind === "placing" && (
              <p className="mb-2 rounded-(--radius-control) bg-warning-50 px-3 py-2 text-sm font-semibold text-warning-600">
                {t.competencies.placing(skillName(gesture.skillId))}
              </p>
            )}
            <div className="relative inline-block max-w-full overflow-auto border border-hairline">
              <canvas ref={canvasRef} className="block" />
              <div
                ref={overlayRef}
                className="absolute inset-0"
                style={{
                  cursor: gesture.kind === "placing" ? "crosshair" : "default",
                }}
                onMouseDown={onMouseDown}
                onMouseMove={onMouseMove}
                onMouseUp={finishGesture}
                onMouseLeave={finishGesture}
              >
                {pageBoxes.map((box) => (
                  <div
                    key={box.skillId}
                    className={`absolute border-2 ${
                      draft?.skillId === box.skillId
                        ? "border-warning-600 bg-warning-600/10"
                        : "border-abysse-600 bg-abysse-600/10"
                    }`}
                    style={{
                      left: box.x,
                      top: box.y,
                      width: box.width,
                      height: box.height,
                      cursor: "move",
                    }}
                  >
                    <span className="absolute -top-5 left-0 max-w-40 truncate rounded bg-abysse-600 px-1 text-[10px] font-semibold text-white">
                      {skillName(box.skillId)}
                    </span>
                    {(["nw", "ne", "sw", "se"] as const).map((corner) => (
                      <span
                        key={corner}
                        className="absolute h-2.5 w-2.5 rounded-full border border-white bg-abysse-600"
                        style={{
                          left: corner.includes("w") ? -5 : undefined,
                          right: corner.includes("e") ? -5 : undefined,
                          top: corner.includes("n") ? -5 : undefined,
                          bottom: corner.includes("s") ? -5 : undefined,
                          cursor: corner === "nw" || corner === "se" ? "nwse-resize" : "nesw-resize",
                        }}
                      />
                    ))}
                  </div>
                ))}
              </div>
            </div>
            <p className="mt-2 text-xs text-ink-soft">{t.competencies.editorHelp}</p>
          </div>

          <div className="max-h-[70vh] space-y-4 overflow-y-auto">
            <Card>
              <h3 className="mb-2 text-sm font-bold">
                ⏳ {t.competencies.positionsUnplaced(unplaced.length)}
              </h3>
              <ul className="space-y-1">
                {unplaced.map((skill) => (
                  <li key={skill.id} className="flex items-center justify-between gap-2">
                    <span className="text-sm">{skill.name}</span>
                    <Button
                      variant="secondary"
                      disabled={gesture.kind === "placing"}
                      onClick={() => setGesture({ kind: "placing", skillId: skill.id })}
                    >
                      📍 {t.competencies.place}
                    </Button>
                  </li>
                ))}
              </ul>
            </Card>
            <Card>
              <h3 className="mb-2 text-sm font-bold">
                ✓ {t.competencies.positionsPlaced(allPositions.length)}
              </h3>
              <ul className="space-y-1">
                {allPositions.map((position) => (
                  <li
                    key={position.skill_id}
                    className="flex items-center justify-between gap-2"
                  >
                    <span className="text-sm">
                      {skillName(position.skill_id)}
                      <span className="ml-1 text-xs text-ink-soft">
                        p.{position.page} • {Math.round(position.x)},{Math.round(position.y)}
                      </span>
                      {position.out_of_range && (
                        <Badge tone="danger">{t.competencies.outOfRange}</Badge>
                      )}
                    </span>
                    <span className="flex shrink-0 gap-1">
                      <button
                        className="min-h-9 px-1.5 text-sm"
                        title={t.competencies.place}
                        onClick={() =>
                          setGesture({ kind: "placing", skillId: position.skill_id })
                        }
                      >
                        📍
                      </button>
                      <button
                        className="min-h-9 px-1.5 text-sm"
                        title={t.competencies.delete}
                        disabled={removePosition.isPending}
                        onClick={() => removePosition.mutate(position.skill_id)}
                      >
                        🗑
                      </button>
                    </span>
                  </li>
                ))}
              </ul>
            </Card>
          </div>
        </div>
      )}
    </>
  );
}
