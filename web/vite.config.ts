import { defineConfig, type Plugin } from "vite";

// Solo `vite dev`: recibe los avisos de pérdidas del motor (src/lossmonitor.ts)
// y los imprime en la terminal donde corre vite.
function lossLog(): Plugin {
  return {
    name: "obrero-loss-log",
    apply: "serve",
    configureServer(server) {
      server.middlewares.use("/__obrero/loss", (req, res) => {
        let body = "";
        req.on("data", (chunk) => (body += chunk));
        req.on("end", () => {
          try {
            const { text } = JSON.parse(body) as { text: string };
            server.config.logger.warn(`[obrero] PÉRDIDA: ${text}`, { timestamp: true });
          } catch {
            // Cuerpo inválido: se ignora.
          }
          res.statusCode = 204;
          res.end();
        });
      });
    },
  };
}

export default defineConfig({ plugins: [lossLog()] });
