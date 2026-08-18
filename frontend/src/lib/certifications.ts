// FFESSM certification catalog for UI pickers. Mirror of the backend ffessm
// crate's FROZEN code list (golden-tested there) — safe to duplicate, codes
// never change. Labels are display-only.
export interface CertificationRef {
  code: string;
  label: string;
  group: string;
}

const G = {
  youth: "Jeunes plongeurs",
  diver: "Plongeur",
  gas: "Nitrox / Trimix",
  handi: "Handisub",
  teaching: "Encadrement fédéral",
  state: "Diplômes d'État",
  club: "Vie du club",
};

export const CERTIFICATIONS: CertificationRef[] = [
  { code: "BRONZE", label: "Plongeur de bronze", group: G.youth },
  { code: "ARGENT", label: "Plongeur d'argent", group: G.youth },
  { code: "OR", label: "Plongeur d'or", group: G.youth },
  { code: "PE12", label: "PE12", group: G.diver },
  { code: "N1", label: "Niveau 1", group: G.diver },
  { code: "PA12", label: "PA12", group: G.diver },
  { code: "PE40", label: "PE40", group: G.diver },
  { code: "PA20", label: "PA20", group: G.diver },
  { code: "N2", label: "Niveau 2", group: G.diver },
  { code: "PA40", label: "PA40", group: G.diver },
  { code: "PE60", label: "PE60", group: G.diver },
  { code: "N3", label: "Niveau 3", group: G.diver },
  { code: "N4", label: "Niveau 4 - Guide de palanquée", group: G.diver },
  { code: "N5", label: "Niveau 5 - Directeur de plongée", group: G.diver },
  { code: "PESH6", label: "PESH6", group: G.handi },
  { code: "PESH12", label: "PESH12", group: G.handi },
  { code: "PESH20", label: "PESH20", group: G.handi },
  { code: "PESH40", label: "PESH40", group: G.handi },
  { code: "NITROX", label: "Plongeur Nitrox de base", group: G.gas },
  { code: "NITROX_C", label: "Plongeur Nitrox confirmé", group: G.gas },
  { code: "TRIMIX_E", label: "Plongeur Trimix élémentaire", group: G.gas },
  { code: "TRIMIX", label: "Plongeur Trimix", group: G.gas },
  { code: "INITIATEUR", label: "Initiateur", group: G.teaching },
  { code: "MF1", label: "Moniteur fédéral 1er degré", group: G.teaching },
  { code: "MF2", label: "Moniteur fédéral 2e degré", group: G.teaching },
  { code: "EH1", label: "Enseignant Handisub EH1", group: G.handi },
  { code: "EH2", label: "Enseignant Handisub EH2", group: G.handi },
  { code: "BEPPA", label: "BEPPA", group: G.state },
  { code: "BPJEPS", label: "BPJEPS plongée", group: G.state },
  { code: "BEES1", label: "BEES 1 plongée", group: G.state },
  { code: "BEES2", label: "BEES 2 plongée", group: G.state },
  { code: "DEJEPS", label: "DEJEPS plongée", group: G.state },
  { code: "DEJEPS_PP", label: "DEJEPS + CC plongée profonde", group: G.state },
  { code: "DESJEPS", label: "DESJEPS plongée", group: G.state },
  { code: "RIFAP", label: "RIFA Plongée", group: G.club },
  { code: "TIV", label: "Technicien en inspection visuelle", group: G.club },
  { code: "PERMIS_COTIER", label: "Permis bateau (option côtière)", group: G.club },
];

export const CERTIFICATION_GROUPS = Object.values(G);

export function certificationLabel(code: string): string {
  return CERTIFICATIONS.find((c) => c.code === code)?.label ?? code;
}
