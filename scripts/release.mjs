const apiBaseUrl = "https://api.github.com";
const uploadsBaseUrl = "https://uploads.github.com";

async function getAssetSign(url) {
  const response = await fetch(url, {
    method: "GET",
    headers: {
      "Content-Type": "application/octet-stream",
    },
  });

  if (!response.ok) {
    throw new Error(`Asset signature download failed: ${response.status} ${await response.text()}`);
  }

  return response.text();
}

async function githubRequest(path, { method = "GET", body, raw = false, upload = false } = {}) {
  const response = await fetch(`${upload ? uploadsBaseUrl : apiBaseUrl}${path}`, {
    method,
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: `Bearer ${process.env.GITHUB_TOKEN}`,
      "X-GitHub-Api-Version": "2022-11-28",
      ...(body ? { "Content-Type": raw ? "application/octet-stream" : "application/json" } : {}),
    },
    body: body ? (raw ? body : JSON.stringify(body)) : undefined,
  });

  if (!response.ok) {
    throw new Error(`GitHub API ${method} ${path} failed: ${response.status} ${await response.text()}`);
  }

  if (response.status === 204) {
    return undefined;
  }

  return response.json();
}

async function createTauriRelease() {
  if (process.env.GITHUB_TOKEN === undefined) {
    throw new Error("GITHUB_TOKEN is not found!");
  }

  if (process.env.GITHUB_REPOSITORY === undefined) {
    throw new Error("GITHUB_REPOSITORY is not found!");
  }

  const [owner, repo] = process.env.GITHUB_REPOSITORY.split("/");
  const repoPath = `/repos/${owner}/${repo}`;
  const tags = await githubRequest(`${repoPath}/tags?per_page=10&page=1`);
  const latestTag = tags.find((tag) => tag.name.startsWith("v"));
  if (!latestTag) {
    throw new Error("No version tag starting with v was found.");
  }
  console.log(latestTag);

  const latestRelease = await githubRequest(`${repoPath}/releases/tags/${latestTag.name}`);
  const latestAssets = latestRelease.assets;

  const windowsX86_64 = {};
  const linuxX86_64 = {};
  const darwinX86_64 = {};
  const darwinAarch64 = {};

  const promises = latestAssets.map(async (asset) => {
    const { name, browser_download_url } = asset;

    if (/\.msi\.zip$/.test(name)) {
      windowsX86_64.url = browser_download_url;
    }
    if (/\.msi\.zip\.sig$/.test(name)) {
      windowsX86_64.signature = await getAssetSign(browser_download_url);
    }

    if (/\.AppImage\.tar\.gz$/.test(name)) {
      linuxX86_64.url = browser_download_url;
    }
    if (/\.AppImage\.tar\.gz\.sig$/.test(name)) {
      linuxX86_64.signature = await getAssetSign(browser_download_url);
    }

    if (/universal\.app\.tar\.gz$/.test(name)) {
      darwinX86_64.url = browser_download_url;
    }
    if (/universal\.app\.tar\.gz\.sig$/.test(name)) {
      darwinX86_64.signature = await getAssetSign(browser_download_url);
    }

    if (/universal\.app\.tar\.gz$/.test(name)) {
      darwinAarch64.url = browser_download_url;
    }
    if (/universal\.app\.tar\.gz\.sig$/.test(name)) {
      darwinAarch64.signature = await getAssetSign(browser_download_url);
    }
  });

  await Promise.allSettled(promises);

  const releaseData = {
    name: latestTag.name,
    notes: `https://github.com/${owner}/${repo}/releases/tag/${latestTag.name}`,
    pub_date: new Date().toISOString(),
    platforms: {},
  };

  if (windowsX86_64.url) releaseData.platforms["windows-x86_64"] = windowsX86_64;
  else console.error('Failed to get release for windowsX86_64');

  if (linuxX86_64.url) releaseData.platforms["linux-x86_64"] = linuxX86_64;
  else console.error('Failed to get release for linuxX86_64');

  if (darwinX86_64.url) releaseData.platforms["darwin-x86_64"] = darwinX86_64;
  else console.error('Failed to get release for darwinX86_64');

  if (darwinAarch64.url) releaseData.platforms["darwin-aarch64"] = darwinAarch64;
  else console.error('Failed to get release for darwinAarch64');

  const tauriRelease = await githubRequest(`${repoPath}/releases/tags/tauri`);

  const prevReleaseAsset = tauriRelease.assets.find((asset) => asset.name === 'release.json');
  if (prevReleaseAsset) {
    await githubRequest(`${repoPath}/releases/assets/${prevReleaseAsset.id}`, {
      method: "DELETE",
    });
  }

  console.log(releaseData);
  await githubRequest(`${repoPath}/releases/${tauriRelease.id}/assets?name=release.json`, {
    method: "POST",
    body: JSON.stringify(releaseData, null, 2),
    raw: true,
    upload: true,
  });
}
createTauriRelease();
