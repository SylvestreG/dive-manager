// Platform admin: the GLOBAL competency referential (level → domain →
// module → skill). Stages and validator grades are fixed rules — nothing to
// configure here. Deletions show the real cascade impact first.
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Navigate } from "react-router";

import { pushErrorToast } from "../../components/toasts";
import {
  Badge,
  Button,
  Card,
  EmptyState,
  ErrorState,
  Field,
  Loading,
  PageTitle,
  Select,
  TextInput,
} from "../../components/ui";
import { t } from "../../i18n/fr";
import { api, type TreeModule, type TreeSkill } from "../../lib/api";
import { translateApiError } from "../../lib/errors";
import { useMe } from "../../lib/session";

function AddForm({
  placeholder,
  withDescription,
  onSubmit,
  pending,
}: {
  placeholder: string;
  withDescription?: boolean;
  onSubmit: (name: string, description: string | null) => void;
  pending: boolean;
}) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  return (
    <form
      className="mt-2 flex flex-wrap items-center gap-2"
      onSubmit={(e) => {
        e.preventDefault();
        if (!name.trim()) return;
        onSubmit(name.trim(), description.trim() || null);
        setName("");
        setDescription("");
      }}
    >
      <TextInput
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder={placeholder}
        className="!w-auto min-w-40 flex-1"
      />
      {withDescription && (
        <TextInput
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder={t.competencies.skillDescription}
          className="!w-auto min-w-40 flex-1"
        />
      )}
      <Button type="submit" variant="secondary" disabled={pending}>
        {t.common.create}
      </Button>
    </form>
  );
}

export default function CompetenciesAdmin() {
  const { data: me } = useMe();
  const queryClient = useQueryClient();
  const [level, setLevel] = useState("N1");
  const [editing, setEditing] = useState<{ id: string; name: string } | null>(
    null,
  );

  const catalog = useQuery({
    queryKey: ["catalog", "certifications"],
    queryFn: () => api.certificationCatalog(),
    staleTime: 30 * 60 * 1000,
  });
  const tree = useQuery({
    queryKey: ["competencies", "tree", level],
    queryFn: () => api.competencies.tree(level),
  });

  const refresh = () => {
    queryClient.invalidateQueries({ queryKey: ["competencies", "tree", level] });
    queryClient.invalidateQueries({ queryKey: ["competencies", "levels"] });
  };
  const onError = (e: unknown) => pushErrorToast(translateApiError(e));

  const createDomain = useMutation({
    meta: { inlineError: true },
    mutationFn: (name: string) =>
      api.competencies.createDomain({ certification: level, name }),
    onSuccess: refresh,
    onError,
  });
  const createModule = useMutation({
    meta: { inlineError: true },
    mutationFn: (body: { domain_id: string; name: string }) =>
      api.competencies.createModule(body),
    onSuccess: refresh,
    onError,
  });
  const createSkill = useMutation({
    meta: { inlineError: true },
    mutationFn: (body: {
      module_id: string;
      name: string;
      description: string | null;
    }) => api.competencies.createSkill(body),
    onSuccess: refresh,
    onError,
  });
  const rename = useMutation({
    meta: { inlineError: true },
    mutationFn: async ({
      kind,
      id,
      name,
      extra,
    }: {
      kind: "domain" | "module" | "skill";
      id: string;
      name: string;
      extra: { sort_order: number; description?: string | null };
    }) => {
      if (kind === "domain") {
        return api.competencies.updateDomain(id, {
          certification: level,
          name,
          sort_order: extra.sort_order,
        });
      }
      if (kind === "module") {
        return api.competencies.updateModule(id, {
          name,
          sort_order: extra.sort_order,
        });
      }
      return api.competencies.updateSkill(id, {
        name,
        description: extra.description ?? null,
        sort_order: extra.sort_order,
      });
    },
    onSuccess: () => {
      setEditing(null);
      refresh();
    },
    onError,
  });
  const removeNode = useMutation({
    meta: { inlineError: true },
    mutationFn: async ({
      kind,
      id,
      name,
    }: {
      kind: "domain" | "module" | "skill";
      id: string;
      name: string;
    }) => {
      // Preflight: show the real cascade impact before confirming.
      const impact =
        kind === "domain"
          ? await api.competencies.domainImpact(id)
          : kind === "module"
            ? await api.competencies.moduleImpact(id)
            : await api.competencies.skillImpact(id);
      const kindLabel =
        kind === "domain"
          ? t.competencies.domain
          : kind === "module"
            ? t.competencies.module
            : t.competencies.skill;
      const message = `${t.competencies.deleteTreeConfirm(kindLabel.toLowerCase(), name)}\n\n${t.competencies.deleteImpact(impact.validations_count)}`;
      if (!window.confirm(message)) return null;
      if (kind === "domain") return api.competencies.deleteDomain(id);
      if (kind === "module") return api.competencies.deleteModule(id);
      return api.competencies.deleteSkill(id);
    },
    onSuccess: refresh,
    onError,
  });

  if (me && (me.impersonating || !me.user.is_global_admin)) {
    return <Navigate to="/" replace />;
  }
  if (catalog.isPending || tree.isPending) return <Loading />;
  if (catalog.isError || tree.isError || !tree.data) {
    return <ErrorState onRetry={() => tree.refetch()} />;
  }
  const rules = tree.data.level;

  const editRow = (
    kind: "domain" | "module" | "skill",
    node: { id: string; name: string; sort_order: number } & Partial<TreeSkill>,
  ) => {
    if (editing?.id !== node.id) return null;
    return (
      <form
        className="mt-1 flex flex-wrap items-center gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          rename.mutate({
            kind,
            id: node.id,
            name: editing.name,
            extra: {
              sort_order: node.sort_order,
              description: (node as TreeSkill).description ?? null,
            },
          });
        }}
      >
        <TextInput
          value={editing.name}
          onChange={(e) => setEditing({ id: node.id, name: e.target.value })}
          className="!w-auto flex-1"
        />
        <Button type="submit" variant="secondary" disabled={rename.isPending}>
          {t.common.save}
        </Button>
        <Button type="button" variant="ghost" onClick={() => setEditing(null)}>
          {t.common.cancel}
        </Button>
      </form>
    );
  };

  const nodeActions = (
    kind: "domain" | "module" | "skill",
    node: { id: string; name: string },
  ) => (
    <span className="flex shrink-0 gap-1">
      <button
        className="min-h-9 rounded-(--radius-control) px-2 text-xs font-semibold text-abysse-700 hover:bg-abysse-50"
        onClick={() => setEditing({ id: node.id, name: node.name })}
      >
        {t.competencies.edit}
      </button>
      <button
        className="min-h-9 rounded-(--radius-control) px-2 text-xs font-semibold text-danger-600 hover:bg-danger-50"
        disabled={removeNode.isPending}
        onClick={() => removeNode.mutate({ kind, id: node.id, name: node.name })}
      >
        {t.competencies.delete}
      </button>
    </span>
  );

  const moduleBlock = (module: TreeModule) => (
    <div key={module.id} className="mt-3 rounded-(--radius-control) border border-hairline p-3">
      <div className="flex items-center justify-between gap-2">
        <h4 className="text-sm font-semibold">📖 {module.name}</h4>
        {nodeActions("module", module)}
      </div>
      {editRow("module", module)}
      <ul className="mt-1">
        {module.skills.map((skill) => (
          <li key={skill.id} className="border-t border-hairline py-1.5 first:border-t-0">
            <div className="flex items-center justify-between gap-2">
              <span className="text-sm">{skill.name}</span>
              {nodeActions("skill", skill)}
            </div>
            {skill.description && (
              <p className="text-xs italic text-ink-soft">💡 {skill.description}</p>
            )}
            {editRow("skill", skill)}
          </li>
        ))}
      </ul>
      <AddForm
        placeholder={t.competencies.addSkill}
        withDescription
        pending={createSkill.isPending}
        onSubmit={(name, description) =>
          createSkill.mutate({ module_id: module.id, name, description })
        }
      />
    </div>
  );

  return (
    <>
      <PageTitle>{t.competencies.adminTitle}</PageTitle>
      <p className="mb-4 text-sm text-ink-soft">{t.competencies.adminHint}</p>

      <Card className="mb-4">
        <Field label={t.competencies.level}>
          <Select value={level} onChange={(e) => setLevel(e.target.value)}>
            {catalog.data.map((entry) => (
              <option key={entry.code} value={entry.code}>
                {entry.label}
              </option>
            ))}
          </Select>
        </Field>
        <div className="mt-2 flex flex-wrap gap-1.5">
          <Badge tone="primary">
            {t.competencies.requiredGrade(rules.required_validator_grade)}
          </Badge>
          {rules.sea_validation && (
            <Badge tone="warning">{t.competencies.seaRule}</Badge>
          )}
        </div>
      </Card>

      {tree.data.domains.length === 0 && (
        <EmptyState message={t.competencies.emptyNoTree} />
      )}
      <div className="space-y-4">
        {tree.data.domains.map((domain) => (
          <Card key={domain.id}>
            <div className="flex items-center justify-between gap-2">
              <h3 className="text-sm font-bold uppercase tracking-wide text-abysse-700">
                📂 {domain.name}
              </h3>
              {nodeActions("domain", domain)}
            </div>
            {editRow("domain", domain)}
            {domain.modules.map(moduleBlock)}
            <AddForm
              placeholder={t.competencies.addModule}
              pending={createModule.isPending}
              onSubmit={(name) =>
                createModule.mutate({ domain_id: domain.id, name })
              }
            />
          </Card>
        ))}
      </div>
      <Card className="mt-4">
        <AddForm
          placeholder={t.competencies.addDomain}
          pending={createDomain.isPending}
          onSubmit={(name) => createDomain.mutate(name)}
        />
      </Card>
    </>
  );
}
