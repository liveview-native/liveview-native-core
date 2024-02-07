defmodule TestServerWeb.ThermostatLive do
  # In Phoenix v1.6+ apps, the line is typically:
  use TestServerWeb, :live_view
  use Phoenix.LiveView
  def render(assigns) do
    ~H"""
    Current temperature: <%= @temperature %>°F
    <button phx-click="inc_temperature">+</button>
    <%= for temp  <- @temperatures do %>
      <p> temp: <%= temp %> </p>
    <% end %>
    """
  end
  def mount(_params, _session, socket) do
    temperature = 70
    temperatures = []
    # Let's assume a fixed temperature for now
    {:ok,
      socket
      |> assign(:temperature, temperature)
      |> assign(:temperatures, temperatures)
    }
  end
  def handle_event("inc_temperature", _params, socket) do
    {:noreply,
      socket
      |> update(:temperature, &(&1 + 1))
      |> update(:temperatures, &(&1 ++ [1]))
    }
  end
end
