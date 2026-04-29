<?php

namespace Example\Service;

// User service.
class Service
{
    // Load one user.
    public function load(string $id): User
    {
        return new User($id);
    }

    private function cacheKey(string $id): string
    {
        return $id;
    }
}

interface UserPort
{
    public function find(string $id): User;
}
