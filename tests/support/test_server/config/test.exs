import Config

# We don't run a server during test. If one is required,
# you can enable the server option below.
config :test_server, TestServerWeb.Endpoint,
  http: [ip: {127, 0, 0, 1}, port: 4002],
  secret_key_base: "DDssS1iT6DUJCZDjdF8G96oAe0ltmvXxizf1ll50h6WVtu90IarmexIMkrfDl79x",
  server: false

# In test we don't send emails.
config :test_server, TestServer.Mailer, adapter: Swoosh.Adapters.Test

# Disable swoosh api client as it is only required for production adapters.
config :swoosh, :api_client, false

# Print only warnings and errors during test
config :logger, level: :warning

# Initialize plugs at runtime for faster test compilation
config :phoenix, :plug_init_mode, :runtime
