
defmodule TestServerWeb.SongPublisher do
  use GenServer
  alias TestServer.{Song}

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, %{})
  end

  @impl true
  def init(_) do
    :timer.send_interval(10000, :update)
    {:ok, 1}
  end

  @impl true
  def handle_info(:update, count)
  do
    new_song = %Song{id: count, title: "song #{count}"}
    Phoenix.PubSub.broadcast(TestServer.PubSub, "songs", new_song)
    IO.puts("Sending song for count #{count}")
    {:noreply, count + 1}
  end
end
