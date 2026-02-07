
const fs = require('fs');
const path = require('path');

const versionTag = process.argv[2];

if (!versionTag) {
    console.error('Usage: node set-version.js <version-tag>');
    process.exit(1);
}

// Expected format: launcher-vX.X.X
const versionMatch = versionTag.match(/^launcher-v(\d+\.\d+\.\d+)$/);

if (!versionMatch) {
    console.error(`Error: Invalid version tag format "${versionTag}". Expected "launcher-vX.X.X".`);
    process.exit(1);
}

const version = versionMatch[1];
console.log(`Derived version: ${version}`);

const launcherDir = path.join(__dirname, '..', 'apps', 'launcher');
const packageJsonPath = path.join(launcherDir, 'package.json');
const tauriConfPath = path.join(launcherDir, 'src-tauri', 'tauri.conf.json');

// Update package.json
if (fs.existsSync(packageJsonPath)) {
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
    console.log(`Updating package.json version from ${packageJson.version} to ${version}`);
    packageJson.version = version;
    fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2) + '\n');
} else {
    console.error(`Error: package.json not found at ${packageJsonPath}`);
    process.exit(1);
}

// Update tauri.conf.json
if (fs.existsSync(tauriConfPath)) {
    const tauriConf = JSON.parse(fs.readFileSync(tauriConfPath, 'utf8'));
    console.log(`Updating tauri.conf.json version from ${tauriConf.version} to ${version}`);
    tauriConf.version = version;
    fs.writeFileSync(tauriConfPath, JSON.stringify(tauriConf, null, 2) + '\n');
} else {
    console.error(`Error: tauri.conf.json not found at ${tauriConfPath}`);
    process.exit(1);
}

console.log('Version update complete.');
