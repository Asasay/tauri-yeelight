import { useYeelight } from "./useYeelight";
import { ConnectionCard } from "./components/ConnectionCard";
import { MainControls } from "./components/MainControls";
import { PresetsCard } from "./components/PresetsCard";
import { AmbientCard } from "./components/AmbientCard";
import { ResponseCard } from "./components/ResponseCard";

export function App() {
  const [state, actions] = useYeelight();
  const connectionReady = state.ip.trim().length > 0 && state.token.trim().length > 0;

  const transition = {
    effect: state.gentleTransitions ? "smooth" : "sudden",
    duration: state.gentleTransitions ? 500 : 30,
  };

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-5xl flex-col gap-4 p-4 md:p-8">
      <ConnectionCard
        ip={state.ip}
        token={state.token}
        port={state.port}
        busy={state.busy}
        setIp={actions.setIp}
        setToken={actions.setToken}
        setPort={actions.setPort}
        onRunDiagnostics={actions.runDiagnostics}
      />

      <div className="grid gap-4 lg:grid-cols-3">
        <MainControls
          brightness={state.brightness}
          ct={state.ct}
          moonlight={state.moonlight}
          busy={state.busy}
          connectionReady={connectionReady}
          transition={transition}
          setBrightness={actions.setBrightness}
          setCt={actions.setCt}
          setMoonlight={actions.setMoonlight}
          onCommand={actions.sendCommand}
        />

        <PresetsCard
          gentleTransitions={state.gentleTransitions}
          busy={state.busy}
          connectionReady={connectionReady}
          onToggleTransitions={actions.setGentleTransitions}
          onCommand={actions.sendCommand}
        />

        <AmbientCard
          ambientOn={state.ambientOn}
          ambientBrightness={state.ambientBrightness}
          busy={state.busy}
          connectionReady={connectionReady}
          transition={transition}
          setAmbientOn={actions.setAmbientOn}
          setAmbientBrightness={actions.setAmbientBrightness}
          onCommand={actions.sendCommand}
        />
      </div>

      <ResponseCard status={state.status} />
    </main>
  );
}
