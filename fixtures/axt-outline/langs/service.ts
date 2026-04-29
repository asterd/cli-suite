import type { User } from "./types";

// User service contract.
export interface UserService {
  // Load one user.
  load(id: string): Promise<User>;
}

// Concrete user service.
export class ApiUserService implements UserService {
  public async load(id: string): Promise<User> {
    return { id };
  }

  private cacheKey(id: string): string {
    return id;
  }
}

// Build a service.
export function createService(): UserService {
  return new ApiUserService();
}

export type UserId = string;

export const DEFAULT_USER_ID = "root";
