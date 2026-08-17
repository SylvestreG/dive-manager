// Small per-user UI preferences (localStorage). Keys embed the user id so
// nothing leaks between accounts on a shared device.

function lastClubKey(userId: string) {
  return `dm_last_club_${userId}`;
}

export function getLastClubSlug(userId: string): string | null {
  return localStorage.getItem(lastClubKey(userId));
}

export function setLastClubSlug(userId: string, slug: string) {
  localStorage.setItem(lastClubKey(userId), slug);
}
