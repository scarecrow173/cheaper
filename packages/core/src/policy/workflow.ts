import { CheaperError } from "../errors.js";

export interface WorkflowArtifactPolicy {
  maxArtifactBytes: number;
  maxTotalBytes: number;
  maxPreviewBytes: number;
  maxReadBytes: number;
}
export interface WorkflowPolicy {
  maxJobs: number;
  artifacts: WorkflowArtifactPolicy;
}
export interface WorkflowPolicyInput {
  maxJobs?: number;
  artifacts?: Partial<WorkflowArtifactPolicy>;
}
const DEFAULT_ARTIFACTS: Readonly<WorkflowArtifactPolicy> = Object.freeze({
  maxArtifactBytes: 8388608,
  maxTotalBytes: 33554432,
  maxPreviewBytes: 2048,
  maxReadBytes: 65536,
});
const cap = (minimum: number) => ({
  type: "integer",
  minimum,
  maximum: Number.MAX_SAFE_INTEGER,
});
export const WORKFLOW_POLICY_PROPERTIES = {
  maxJobs: cap(1),
  artifacts: {
    type: "object",
    additionalProperties: false,
    properties: {
      maxArtifactBytes: cap(0),
      maxTotalBytes: cap(0),
      maxPreviewBytes: cap(0),
      maxReadBytes: cap(1),
    },
  },
};

/** trusted merge 後・project restriction 前の defaults と runtime options の正本。 */
export function resolveWorkflowPolicy(input?: WorkflowPolicyInput): WorkflowPolicy {
  const maxJobs = input?.maxJobs ?? 1;
  const artifacts = { ...DEFAULT_ARTIFACTS, ...input?.artifacts };
  if (!Number.isSafeInteger(maxJobs) || maxJobs < 1) {
    throw new CheaperError("INVALID_RUNTIME_POLICY", "invalid workflow.maxJobs");
  }
  for (const [field, value] of Object.entries(artifacts)) {
    if (!Number.isSafeInteger(value) || value < (field === "maxReadBytes" ? 1 : 0)) {
      throw new CheaperError("INVALID_RUNTIME_POLICY", `invalid workflow.artifacts.${field}`);
    }
  }
  return { maxJobs, artifacts };
}
