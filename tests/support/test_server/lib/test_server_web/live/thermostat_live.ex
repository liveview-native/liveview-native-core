defmodule TestServerWeb.ThermostatLive do
  use Phoenix.LiveView
  use TestServerWeb, :live_view
  use TestServerNative, :live_view
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
  def render(%{} = assigns) do
    ~H"""
    Current temperature: <%= @temperature %>°F
    <button phx-click="inc_temperature">+</button>
    <%= for temp  <- @temperatures do %>
      <p> temp: <%= temp %> </p>
    <% end %>
    """
  end
end
defmodule TestServerWeb.ThermostatLive.SwiftUI do
  use TestServerNative, [:render_component, format: :swiftui]

  def render(assigns, _interface) do
    ~LVN"""
    <VStack>
      <Text>
        Hello SwiftUI!
      </Text>
    </VStack>
    """
  end
end
defmodule TestServerWeb.ThermostatLive.Jetpack do
  use TestServerNative, [:render_component, format: :jetpack]

  def render(assigns, _) do
    ~LVN"""
    <Box size="fill" background="system-blue">
      <Text align="Center">Hello</Text>
    </Box>
    """
  end
end
