defmodule TestServerWeb.SimpleLiveStream do
  use TestServerWeb, :live_view
  use TestServerNative, :live_view
  alias TestServer.Song

  def mount(_params, _session, socket) do

    Phoenix.PubSub.subscribe(TestServer.PubSub, "songs")

    song_0 = %Song{id: 0, title: "base song 0"}
    song_1 = %Song{id: 1, title: "base song 1"}
    songs = [song_0, song_1]
    songs_other = []

    {:ok,
      socket
      #|> assign(:song_is_even, song_is_even)
      |> stream(:songs, songs)
      |> stream(:songs_other, songs_other)
    }
  end

  def handle_info(song, socket) do
    {:noreply,
      socket
      #|> update(:song_is_even, song_is_even)
      |> stream_insert(:songs, song)
      |> stream_insert(:songs_other, song)
    }
  end

  def handle_event("delete-song", %{"id" => id}, socket) do
    song = %Song{id: id, title: "song #{id}"}
    IO.puts("Deleting song: #{id}")
    {:noreply,
      socket
      |> stream_delete(:songs, song)
    }
  end

  def handle_event("add-fancy-song", %{"id" => id}, socket) do
    song = %Song{id: id, title: "fancy song #{id}"}
    {:noreply,
      socket
      |> stream_insert(:songs, song)
    }
  end

  def handle_event("reset-stream", %{"id" => _id}, socket) do
    song_0 = %Song{id: 0, title: "reset base song 0"}
    song_1 = %Song{id: 1, title: "reset base song 1"}
    songs = [song_0, song_1]
    {:noreply,
      socket
      |> stream(:songs, songs, reset: true)
    }
  end

  def handle_event("increment-song", %{"id" => id}, socket) do
    old_song = %Song{id: id, title: "song #{id}"}
    new_id = String.to_integer(id) + 1
    new_song = %Song{id: id, title: "song #{new_id}"}
    IO.puts("Deleting song: #{id}")
    {:noreply,
      socket
      |> stream_delete(:songs, old_song)
      |> stream_insert(:songs, new_song)
    }
  end

  def render(assigns) do
    ~H"""
    <.link href={~p"/upload"}>Upload Page</.link>
    <button phx-click="reset-stream" phx-value-id="0">Reset Stream</button>
    <button phx-click="add-fancy-song" phx-value-id="9999">Add fancy song</button>
    """
  end
#  def render(assigns) do
#    ~H"""
#    <.link href={~p"/upload"}>Upload Page</.link>
#    <table>
#      <tbody id="songs" phx-update="stream">
#        <tr
#          :for={{id, song} <- @streams.songs}
#            id={id}
#        >
#          <td><%= song.title %></td>
#          <td><button phx-click="delete-song" phx-value-id={song.id}>delete</button></td>
#          <td><button phx-click="increment-song" phx-value-id={song.id}>increment</button></td>
#        </tr>
#      </tbody>
#    </table>
#    <button phx-click="reset-stream" phx-value-id="0">Reset Stream</button>
#    <button phx-click="add-fancy-song" phx-value-id="9999">Add fancy song</button>
#    <table>
#      <tbody id="songs_other" phx-update="stream">
#        <tr
#          :for={{id, song} <- @streams.songs_other}
#            id={id}
#        >
#          <td><%= song.title %></td>
#          <td><button phx-click="delete-song" phx-value-id={song.id}>delete</button></td>
#          <td><button phx-click="increment-song" phx-value-id={song.id}>increment</button></td>
#        </tr>
#      </tbody>
#    </table>
#    """
#  end
end

defmodule TestServerWeb.SimpleLiveStream.SwiftUI do
  use TestServerNative, [:render_component, format: :swiftui]

  def render(assigns, _interface) do
    ~LVN"""
    <Table>
      <tbody id="songs_other" phx-update="stream">
        <tr
          :for={{id, song} <- @streams.songs_other}
            id={id}
        >
          <td><%= song.title %></td>
          <td><button phx-click="delete-song" phx-value-id={song.id}>delete</button></td>
          <td><button phx-click="increment-song" phx-value-id={song.id}>increment</button></td>
        </tr>
      </tbody>
    </Table>
    """
  end
end
defmodule TestServerWeb.SimpleLiveStream.Jetpack do
  use TestServerNative, [:render_component, format: :jetpack]

  def render(assigns, _) do
    ~LVN"""
    <Box size="fill" background="system-blue">
      <Text align="Center">Hello</Text>
    </Box>
    """
  end
end
