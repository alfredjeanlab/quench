// VIOLATION: `as unknown` without // CAST: comment
const data = fetch('/api/data') as unknown as UserData;

interface UserData {
  name: string;
}
