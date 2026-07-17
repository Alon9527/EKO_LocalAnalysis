export function groupMaterialsByCategory<T extends { category: string }>(assets: T[]) {
  return assets.reduce<Record<string, T[]>>((groups, asset) => {
    (groups[asset.category] ||= []).push(asset);
    return groups;
  }, {});
}