// Shared "who is this person" fieldset (email + first/last name) used by every
// invite/add form — labeled fields (accessibility rule), one implementation.
import { t } from "../i18n/fr";
import { Field, TextInput } from "./ui";

export interface PersonDraft {
  email: string;
  first_name: string;
  last_name: string;
}

export const EMPTY_PERSON: PersonDraft = {
  email: "",
  first_name: "",
  last_name: "",
};

export function personComplete(person: PersonDraft): boolean {
  return (
    person.email.trim() !== "" &&
    person.first_name.trim() !== "" &&
    person.last_name.trim() !== ""
  );
}

export function PersonFields({
  value,
  onChange,
}: {
  value: PersonDraft;
  onChange: (value: PersonDraft) => void;
}) {
  return (
    <div className="flex flex-col gap-3">
      <Field label={t.common.email}>
        <TextInput
          type="email"
          inputMode="email"
          autoComplete="off"
          value={value.email}
          onChange={(e) => onChange({ ...value, email: e.target.value })}
        />
      </Field>
      <div className="flex gap-2">
        <Field label={t.common.firstName}>
          <TextInput
            value={value.first_name}
            onChange={(e) => onChange({ ...value, first_name: e.target.value })}
          />
        </Field>
        <Field label={t.common.lastName}>
          <TextInput
            value={value.last_name}
            onChange={(e) => onChange({ ...value, last_name: e.target.value })}
          />
        </Field>
      </div>
    </div>
  );
}
