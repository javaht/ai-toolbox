export interface ClaudeDesktopInferenceModel {
  name: string;
}

export interface ClaudeDesktopProvider {
  id: string;
  name: string;
  inferenceProvider: string;
  inferenceGatewayBaseUrl: string;
  inferenceGatewayApiKey: string;
  inferenceModels: ClaudeDesktopInferenceModel[];
  notes?: string;
  sortIndex?: number;
  isApplied: boolean;
  isDisabled: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface ClaudeDesktopProviderInput {
  id?: string;
  name: string;
  inferenceProvider: string;
  inferenceGatewayBaseUrl: string;
  inferenceGatewayApiKey: string;
  inferenceModels: ClaudeDesktopInferenceModel[];
  notes?: string;
  sortIndex?: number;
}

export interface ClaudeDesktopProviderFormValues {
  name: string;
  inferenceProvider: string;
  inferenceGatewayBaseUrl: string;
  inferenceGatewayApiKey: string;
  inferenceModels: ClaudeDesktopInferenceModel[];
  notes?: string;
}

export interface ClaudeDesktopCommonConfig {
  config: string;
  rootDir?: string | null;
  updatedAt?: string;
}

export interface ClaudeDesktopCommonConfigInput {
  config: string;
  rootDir?: string | null;
  clearRootDir?: boolean;
}

export interface ClaudeDesktopPathInfo {
  path: string;
  source: 'custom' | 'default';
}
