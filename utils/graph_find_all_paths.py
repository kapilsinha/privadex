from collections import defaultdict
  
# Recursive program based on https://www.geeksforgeeks.org/find-paths-given-source-destination/
# Iterative implementation is my own
class Graph:
    def __init__(self, vertices):
        # No. of vertices
        self.V = vertices
         
        # default dictionary to store graph
        self.graph = defaultdict(list)
  
    # function to add an edge to graph
    def add_edge(self, u, v):
        self.graph[u].append(v)
  
    # Prints all paths from 's' to 'd'
    def print_all_paths_rec(self, s, d):
        def print_all_paths_rec_helper(u, d, visited, path):
            '''
            A recursive function to print all paths from 'u' to 'd'.
            visited[] keeps track of vertices in current path.
            path[] stores actual vertices and path_index is current
            index in path[]
            '''
            # Mark the current node as visited and store in path
            visited[u]= True
            path.append(u)
    
            # If current vertex is same as destination, then print
            # current path[]
            if u == d:
                print (path)
            else:
                # If current vertex is not destination
                # Recur for all the vertices adjacent to this vertex
                for i in self.graph[u]:
                    if visited[i]== False:
                        print_all_paths_rec_helper(i, d, visited, path)
                        
            # Remove current vertex from path[] and mark it as unvisited
            path.pop()
            visited[u]= False

        # Mark all the vertices as not visited
        visited =[False]*(self.V)
 
        # Create an array to store paths
        path = []
 
        # Call the recursive helper function to print all paths
        print_all_paths_rec_helper(s, d, visited, path)
  

    def print_all_paths_iter(self, s, d):
        # Mark all the vertices as not visited
        visited =[False]*(self.V)
 
        # Create an array to store paths
        path = []

        stack = [(s, 0)]

        while len(stack) > 0:
            # print(stack, path, visited)
            u, path_len = stack.pop(-1)

            # Trim path and visited
            for i in range(len(path) - path_len):
                excised = path.pop(-1)
                visited[excised] = False

            # Mark the current node as visited and store in path
            path.append(u)
            visited[u]= True
    
            # If current vertex is same as destination, then print
            # current path[]
            if u == d:
                print(path)
            else:
                # If current vertex is not destination
                # Recur for all the vertices adjacent to this vertex
                for i in self.graph[u]:
                    if visited[i] == False:
                        stack.append((i, path_len + 1))
  
# Create a graph given in the above diagram
g = Graph(5)
g.add_edge(0, 1)
g.add_edge(0, 2)
g.add_edge(0, 3)
g.add_edge(1, 0)
g.add_edge(1, 3)
g.add_edge(1, 4)
g.add_edge(2, 0)
g.add_edge(2, 1)
g.add_edge(2, 4)
g.add_edge(3, 1)
g.add_edge(3, 2)
g.add_edge(3, 4)
g.add_edge(4, 1)
  
s = 1
d = 4
print(f"Finding all unique paths from {s} to {d}")
print("Recursive:")
g.print_all_paths_rec(s, d)
print("Iterative:")
g.print_all_paths_iter(s, d)
