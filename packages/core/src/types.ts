// Type definitions for UEPM

export interface UProjectFile {
  EngineAssociation: string;
  Category?: string;
  Description?: string;
  Modules?: Module[];
  Plugins?: PluginReference[];
  AdditionalPluginDirectories?: string[];
  TargetPlatforms?: string[];
  [key: string]: unknown; // Allow additional fields
}

export interface Module {
  Name: string;
  Type: string;
  LoadingPhase: string;
  AdditionalDependencies?: string[];
}

export interface PluginReference {
  Name: string;
  Enabled: boolean;
  MarketplaceURL?: string;
  SupportedTargetPlatforms?: string[];
}

export interface PackageJson {
  name: string;
  version: string;
  description?: string;
  main?: string;
  files?: string[];
  keywords?: string[];
  private?: boolean;
  scripts?: Record<string, string>;
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
  engines?: {
    node?: string;
    npm?: string;
  };
  author?: string;
  homepage?: string;
  license?: string;
  unreal?: {
    engineVersion?: string;
    pluginName?: string;
  };
  [key: string]: unknown; // Allow additional fields
}

export interface UPluginFile {
  FileVersion: number;
  Version?: number;
  VersionName?: string;
  FriendlyName?: string;
  Description?: string;
  Category?: string;
  CreatedBy?: string;
  CreatedByURL?: string;
  DocsURL?: string;
  MarketplaceURL?: string;
  SupportURL?: string;
  EngineVersion?: string;
  CanContainContent?: boolean;
  IsBetaVersion?: boolean;
  IsExperimentalVersion?: boolean;
  Installed?: boolean;
  Modules?: UPluginModule[];
  Plugins?: PluginDependency[];
  [key: string]: unknown; // Allow additional fields
}

export interface UPluginModule {
  Name: string;
  Type: string;
  LoadingPhase?: string;
  AdditionalDependencies?: string[];
  WhitelistPlatforms?: string[];
  BlacklistPlatforms?: string[];
}

export interface PluginDependency {
  Name: string;
  Enabled: boolean;
  Optional?: boolean;
}

export interface InitContext {
  type: 'project' | 'plugin';
  primaryFile: string;
  directory: string;
  pluginName?: string; // Only for plugin context
}

export type ContextDetectionResult =
  | { success: true;  context: InitContext; warnings?: string[] }
  | { success: false; error: string;        warnings?: string[] };
