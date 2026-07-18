import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";

import { strings } from "../../lib/strings";

export function AboutTab() {
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion().then(setVersion);
  }, []);

  return (
    <div className="tab-panel about-tab">
      <img src="/icon.png" alt="" width={64} height={64} />
      <h2>{strings.aboutTab.appName}</h2>
      <p className="version">{version ? `Phiên bản ${version}` : ""}</p>
      <p>{strings.aboutTab.description}</p>
      <p>{strings.aboutTab.credit}</p>
      <button onClick={() => void openUrl(strings.aboutTab.sourceCodeUrl)}>{strings.aboutTab.sourceCode}</button>
    </div>
  );
}
