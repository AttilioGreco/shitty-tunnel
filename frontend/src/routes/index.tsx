import { createFileRoute } from "@tanstack/react-router";
import { InspectorPage } from "@/components/inspector/inspector-page";

export const Route = createFileRoute("/")({ component: App });

function App() {
return (
  <InspectorPage />
);
}
