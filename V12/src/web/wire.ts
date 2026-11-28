// Wire types: the JSON the Rust web backend emits (web/doc-engine, web/server). English, camelCase.
// The only module pair that knows both vocabularies is adapt.ts (wire -> existing desktop types).

export type WireNodeKind = "directory" | "archive" | "file";

export interface WireNode {
  name: string;
  path: string;
  kind: WireNodeKind;
  size: number;
  storedSize: number;
  modified: boolean;
  resource: boolean;
  children: WireNode[];
}

export interface WireTarget {
  source: string;
  archives: string[];
  path: string;
  delete: boolean;
}

export interface WirePackage {
  name: string;
  version: string;
  author: string;
  description: string;
  targets: WireTarget[];
}

export interface WireDocument {
  docId: number;
  name: string;
  encryption: string;
  diskSize: number;
  estimatedSize: number;
  dirty: boolean;
  undoDepth: number;
  redoDepth: number;
  revision: number;
  package: WirePackage | null;
  packageError: string | null;
  missingSources: string[];
  tree: WireNode[];
}

export interface WireFile {
  fileId: string;
  name: string;
  size: number;
}

export interface WireOpenDocument {
  docId: number;
  name: string;
  fileId: string;
}

export interface WireSession {
  limits: {
    maxUploadBytes: number;
    maxWorkspaceBytes: number;
    maxFiles: number;
    maxOpenDocuments: number;
    idleTtlSeconds: number;
    maxAgeSeconds: number;
  };
}

export type WireEntryKind = "meta" | "texture" | "model" | "resource" | "setting" | "text" | "binary";

export interface WireEntryView {
  kind: WireEntryKind;
  size: number;
  text: string | null;
  hexDump: string | null;
  rscVersion: number | null;
}

export interface WireSaveResult {
  summary: WireDocument;
  oldSize: number;
  newSize: number;
  backupId: string | null;
}

export interface WireBackup {
  backupId: string;
  size: number;
  createdUnix: number;
}

export interface WireSettingEntry {
  key: string;
  value: string;
  line: number;
  repeat: number;
}

export interface WireSettingColor {
  base: string;
  keys: [string, string, string];
  scale: number;
}

export interface WireSettings {
  entries: WireSettingEntry[];
  colors: WireSettingColor[];
}
