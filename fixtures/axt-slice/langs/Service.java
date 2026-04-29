package example.slice;

import java.util.Objects;

public class Service {
    public Service() {
    }

    public String process(String value) {
        return Objects.requireNonNull(value).trim();
    }
}
