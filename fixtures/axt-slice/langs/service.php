<?php

namespace Example\Slice;

use DateTimeImmutable;

final class Service
{
    public function __construct()
    {
    }

    public function process(string $value): string
    {
        return (new DateTimeImmutable($value))->format('c');
    }
}
