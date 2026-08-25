import { useEffect, useState } from "react";
import { Sidebar } from "./Sidebar";
import { errorMessage, ipc } from "./ipc";
import { NAV_ITEMS, type ViewKey } from "./nav";
import { useTheme } from "./theme";
import type { AgentInfo, OpcStatus, ProjectInfo, ProviderInfo } from "./types";
import { ComingSoonView } from "./views/ComingSoon";
import { ControlPanel } from "./views/ControlPanel";
import { ModelCenter } from "./views/ModelCenter";
import { ProjectCenter } from "./views/ProjectCenter";
import { TeamCenter } from "./views/TeamCenter";
import { WorkflowCenter } from "./views/WorkflowCenter";

export default function App() {
  const { choice: theme, setChoice: setTheme } = useTheme();
  const [view, setView] = useState<ViewKey>("control-panel");
  const [requestedProjectId, setRequestedProjectId] = useState<string | null>(null);

  const [status, setStatus] = useState<OpcStatus | null>(null);
  const [statusError, setStatusError] = useState<string | null>(null);
  const [providers, setProviders] = useState<ProviderInfo[] | null>(null);
  const [providersError, setProvidersError] = useState<string | null>(null);
  const [agents, setAgents] = useState<AgentInfo[] | null>(null);
  const [agentsError, setAgentsError] = useState<string | null>(null);
  const [projects, setProjects] = useState<ProjectInfo[] | null>(null);
  const [projectsError, setProjectsError] = useState<string | null>(null);

  const refreshProjects = () => ipc.listProjects().then(setProjects).catch((e) => setProjectsError(errorMessage(e)));

  useEffect(() => {
    ipc.status().then(setStatus).catch((e) => setStatusError(errorMessage(e)));
    ipc.providers().then(setProviders).catch((e) => setProvidersError(errorMessage(e)));
    ipc.agents().then(setAgents).catch((e) => setAgentsError(errorMessage(e)));
    refreshProjects();
  }, []);

  const navigate = (v: ViewKey) => setView(v);
  const openWorkflowFor = (projectId: string) => {
    setRequestedProjectId(projectId);
    setView("workflow-center");
  };

  const activeItem = NAV_ITEMS.find((n) => n.key === view)!;

  const counts: Partial<Record<ViewKey, number>> = {
    "project-center": projects?.length,
    "team-center": agents?.length,
    "model-center": providers?.filter((p) => p.available).length,
  };

  let body: React.ReactNode;
  if (!activeItem.ready) {
    body = <ComingSoonView view={view} onNavigate={navigate} />;
  } else if (view === "project-center") {
    body = (
      <ProjectCenter
        projects={projects}
        projectsError={projectsError}
        onCreated={(created) => {
          refreshProjects();
          if (created.active_workflow_id) openWorkflowFor(created.id);
        }}
        onOpenWorkflow={openWorkflowFor}
      />
    );
  } else if (view === "team-center") {
    body = <TeamCenter agents={agents} agentsError={agentsError} />;
  } else if (view === "model-center") {
    body = <ModelCenter providers={providers} providersError={providersError} />;
  } else if (view === "workflow-center") {
    body = <WorkflowCenter projects={projects} agents={agents} requestedProjectId={requestedProjectId} />;
  } else {
    body = (
      <ControlPanel
        status={status}
        statusError={statusError}
        providers={providers}
        agents={agents}
        projects={projects}
        onNavigate={navigate}
      />
    );
  }

  return (
    <div className="flex h-screen overflow-hidden bg-background text-foreground">
      <Sidebar active={view} onSelect={navigate} counts={counts} theme={theme} onThemeChange={setTheme} />
      <main className="flex h-screen flex-1 flex-col overflow-y-auto [&>*]:animate-fade-slide-in">{body}</main>
    </div>
  );
}
