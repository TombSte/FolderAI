function rootLength(path: string) {
  if (path.startsWith('/')) return 1;
  const drive = path.match(/^(?:\\\\\?\\)?[a-z]:[\\/]/i);
  if (drive) return drive[0].length;
  const share = path.match(/^(?:\\\\\?\\UNC\\|\\\\)[^\\/]+[\\/][^\\/]+/i);
  return share?.[0].length ?? 0;
}

function pathParts(path: string) {
  const root = rootLength(path);
  const separators = path.startsWith('/') ? /\/+$/ : /[\\/]+$/;
  const trimmed = path.slice(0, root) + path.slice(root).replace(separators, '');
  const separator = path.startsWith('/') ? trimmed.lastIndexOf('/') : Math.max(trimmed.lastIndexOf('/'), trimmed.lastIndexOf('\\'));
  return { root, trimmed, separator };
}

export function pathLabel(path: string | null) {
  if (!path) return 'Home';
  const { root, trimmed, separator } = pathParts(path);
  return trimmed.length <= root ? trimmed : trimmed.slice(separator + 1) || path;
}

export function parentPath(path: string) {
  const { root, trimmed, separator } = pathParts(path);
  if (!root || trimmed.length <= root) return null;
  return trimmed.slice(0, Math.max(root, separator));
}
