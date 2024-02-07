defmodule TestServer.Application do
  # See https://hexdocs.pm/elixir/Application.html
  # for more information on OTP Applications
  @moduledoc false

  use Application

  @impl true
  def start(_type, _args) do
    children = [
      TestServerWeb.Telemetry,
      {DNSCluster, query: Application.get_env(:test_server, :dns_cluster_query) || :ignore},
      {Phoenix.PubSub, name: TestServer.PubSub},
      # Start the Finch HTTP client for sending emails
      {Finch, name: TestServer.Finch},
      # Start a worker by calling: TestServer.Worker.start_link(arg)
      # {TestServer.Worker, arg},
      # Start to serve requests, typically the last entry
      TestServerWeb.Endpoint,
      TestServerWeb.SongPublisher,
    ]

    # See https://hexdocs.pm/elixir/Supervisor.html
    # for other strategies and supported options
    opts = [strategy: :one_for_one, name: TestServer.Supervisor]
    Supervisor.start_link(children, opts)
  end

  # Tell Phoenix to update the endpoint configuration
  # whenever the application is updated.
  @impl true
  def config_change(changed, _new, removed) do
    TestServerWeb.Endpoint.config_change(changed, removed)
    :ok
  end
end
